use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use tokio::signal;

pub fn socketaddr_to_url(addr: &SocketAddr) -> String {
    match addr.ip() {
        IpAddr::V4(_) => format!("http://{}", addr), // IPv4 address
        IpAddr::V6(ipv6) => {
            // Convert IPv6 loopback to 127.0.0.1 if it's the loopback address
            if ipv6.is_loopback() {
                format!("http://localhost:{}", addr.port())
            } else {
                format!("http://[{}]:{}", ipv6, addr.port())
            }
        }
    }
}

pub async fn listen_shutdown_signal<F>(shutdown_handler: F, delay: Option<Duration>)
where
    F: Fn(Option<Duration>) + Send + 'static,
{
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(windows)]
    let terminate = async {
        signal::windows::ctrl_c()
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => println!("ctrl_c signal received"),
        _ = terminate => println!("terminate signal received"),
    };

    // Graceful Shutdown Server
    shutdown_handler(delay);
}
