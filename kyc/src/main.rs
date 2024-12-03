mod service;
mod transport;

use std::net::SocketAddr;
use transport::grpc::serve;

#[tokio::main]
async fn main() {
    let addr = "[::1]:50051".parse::<SocketAddr>().unwrap();
    match serve(addr).await.err() {
        Some(err) => panic!("{}", err),
        _ => (),
    };
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use crate::transport::grpc::serve;

    #[tokio::test]
    async fn test_yoroi() {
        let addr = "[::1]:50051".parse::<SocketAddr>().unwrap();
        let _ = tokio::spawn(async move { serve(addr).await });
        let mut client = match crate::transport::grpc::kyc_rpc::kyc_client::KycClient::connect(
            yoroi::utils::socketaddr_to_url(&addr),
        )
        .await
        {
            Ok(client) => client,
            Err(err) => panic!("{}", err),
        };
        match client
            .ping(crate::transport::grpc::kyc_rpc::Ping {
                message: "Ping".to_string(),
            })
            .await
        {
            Ok(r) => println!("{:?}", r),
            Err(err) => panic!("{}", err),
        }
    }
}
