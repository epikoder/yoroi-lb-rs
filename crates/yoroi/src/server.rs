use http_body_util::combinators::BoxBody;
use hyper::upgrade::Upgraded;
use hyper::{server::conn::http2, service::service_fn};
use std::sync::Arc;
use std::time::Duration;
use std::{error::Error, net::SocketAddr, str::FromStr};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, error, info};

use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::{Method, Request, Response, StatusCode, Uri};
use hyper_util::rt::TokioIo;

use crate::registry::ServiceRegistry;
use crate::utils;

#[derive(Clone)]
pub struct ShutdownHandler(Sender<Option<usize>>);

impl ShutdownHandler {
    pub async fn graceful_shutdown(&self, delay: Option<Duration>) {
        let sender = self.0.clone();
        if let Some(t) = delay {
            sleep(t).await;
        };
        if let Err(e) = sender.send(Some(1)).await {
            println!("Failed to send shutdown signal: {}", e);
        }
    }
}

#[derive(Clone)]
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

#[derive(Clone)]
pub struct Server {
    sender: Sender<Option<usize>>,
    receiver: Arc<tokio::sync::Mutex<Receiver<Option<usize>>>>,
    pub(crate) registry: ServiceRegistry,
}

impl Default for Server {
    fn default() -> Self {
        Server::new()
    }
}

impl Server {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1);
        Server {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
            registry: ServiceRegistry::new(),
        }
    }

    pub fn handler(&self) -> ShutdownHandler {
        ShutdownHandler(self.sender.clone())
    }

    // start the server
    pub async fn serve(&self, address: &str) -> Result<(), Box<dyn Error>> {
        tracing_subscriber::fmt().init();
        let addr = SocketAddr::from_str(address)
            .map_err(|err| format!("Invalid address: {}: {}", address, err))?;
        let listener = TcpListener::bind(addr).await?;

        let address = address.to_string();

        let shutdown_handler = self.handler();
        tokio::spawn(async move {
            utils::listen_shutdown_signal(
                move |delay: Option<Duration>| {
                    let shutdown_handler = shutdown_handler.clone();
                    tokio::spawn(async move {
                        shutdown_handler.graceful_shutdown(delay).await;
                    });
                },
                None,
            )
            .await;
        });

        let mut tasks: Vec<JoinHandle<()>> = vec![];
        info!("Yoroi started on: {}", address);

        let mut rx = self.receiver.lock().await;
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let io = TokioIo::new(stream);
                            let sreg = self.registry.clone();
                            let task =  tokio::task::spawn(async move {
                                if let Err(err) = http2::Builder::new(TokioExecutor)
                                    .serve_connection(io, service_fn(|req| proxy(sreg.clone(), req)))
                                    .await
                                {
                                    error!("Error serving connection: {}", err);
                                }
                            });
                            tasks.push(task);
                        }
                        Err(err) => {
                            error!("Error accepting connection: {}", err);
                        }
                    }
                }

                _ = rx.recv() => {
                    info!("Shutdown signal received. Breaking loop...");
                    break;
                }
            }
        }
        // Wait for all tasks to finish before shutting down
        for task in tasks {
            let _ = task.await;
        }
        Ok(())
    }
}

async fn proxy(
    sreg: ServiceRegistry,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let mut id = req.uri().path();
    if id.starts_with("/") {
        id = &id[1..];
    }
    id = id.split("/").collect::<Vec<&str>>()[0];

    let endpoint = match sreg.resolve_endpoint(id.to_string()) {
        Some(uri) => uri,
        None => return Ok(Response::new(empty())),
    };

    if Method::CONNECT == req.method() {
        if let Some(addr) = host_addr(req.uri()) {
            tokio::task::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) = tunnel(upgraded, addr).await {
                            error!("server io error: {}", e);
                        };
                    }
                    Err(e) => error!("upgrade error: {}", e),
                }
            });

            Ok(Response::new(empty()))
        } else {
            eprintln!("CONNECT host is not socket addr: {:?}", req.uri());
            let mut resp = Response::new(full("CONNECT must be to a socket address"));
            *resp.status_mut() = StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    } else {
        let stream = match TcpStream::connect(endpoint).await {
            Ok(s) => s,
            Err(err) => panic!("{}", err),
        };
        let io = TokioIo::new(stream);

        let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor)
            .handshake(io)
            .await?;
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

        let resp = sender.send_request(req).await?;
        Ok(resp.map(|b| b.boxed()))
    }
}

fn host_addr(uri: &Uri) -> Option<String> {
    uri.authority().and_then(|auth| Some(auth.to_string()))
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

// Create a TCP connection to host:port, build a tunnel between the connection and
// the upgraded connection
async fn tunnel(upgraded: Upgraded, addr: String) -> std::io::Result<()> {
    // Connect to remote server
    let mut server = TcpStream::connect(addr).await?;
    let mut upgraded = TokioIo::new(upgraded);

    // Proxying data
    let (from_client, from_server) =
        tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;

    // Print message when done
    debug!(
        "client wrote {} bytes and received {} bytes",
        from_client, from_server
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    #[tokio::test]
    async fn test_start_server() {
        let server = Server::new();
        let handler = server.handler();
        tokio::spawn(async move {
            let _ = server.serve("127.0.0.1:8080").await;
        });
        handler
            .graceful_shutdown(Some(Duration::from_secs(3)))
            .await;
    }
}
