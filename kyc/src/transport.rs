pub mod grpc {
    use core::net::SocketAddr;
    use kyc_rpc::kyc_server::{Kyc, KycServer};
    use kyc_rpc::{Ping, Pong, RegisterRequest, RegisterResponse};
    use tonic::transport::Error;
    use tonic::{transport::Server, Request, Response, Status};
    use tracing::info;

    use crate::service::Service;

    pub mod kyc_rpc {
        tonic::include_proto!("kyc");
    }

    #[tonic::async_trait]
    impl Kyc for Service {
        async fn ping(&self, _: Request<Ping>) -> Result<Response<Pong>, Status> {
            let reply = Pong {
                message: "pong".to_string(),
            };

            Ok(Response::new(reply))
        }

        async fn register(
            &self,
            request: Request<RegisterRequest>,
        ) -> Result<Response<RegisterResponse>, Status> {
            let RegisterRequest { email, password } = request.into_inner();
            if let Some(err) = self._register(email, password).await.err() {
                return Err(Status::new(tonic::Code::Aborted, format!("{}", err)));
            }

            let reply = RegisterResponse {
                message: "Account created successfully".to_string(),
            };

            Ok(Response::new(reply))
        }
    }

    pub async fn serve(addr: SocketAddr) -> Result<(), Error> {
        tracing_subscriber::fmt().init();
        let srv = Service::default();
        let mut server = Server::builder();
        info!("Kyc started on:: {}", addr.clone().to_string());
        server
            .add_service(KycServer::new(srv))
            .serve(addr.clone())
            .await
    }

    #[cfg(test)]
    mod tests {

        use std::{net::SocketAddr, time::Duration};

        use tokio::time::sleep;

        use super::*;

        #[tokio::test]
        async fn ping() {
            let addr = "[::1]:50051".parse::<SocketAddr>().unwrap();

            tokio::spawn(async move {
                let _ = serve(addr).await;
            });

            sleep(Duration::from_millis(100)).await;
            let mut client = match kyc_rpc::kyc_client::KycClient::connect(
                yoroi::utils::socketaddr_to_url(&addr),
            )
            .await
            {
                Ok(client) => client,
                Err(err) => panic!("{}", err),
            };
            match client
                .ping(Ping {
                    message: "Ping".to_string(),
                })
                .await
            {
                Ok(r) => println!("{:?}", r),
                Err(err) => panic!("{}", err),
            }
        }
    }
}

pub mod http {}
