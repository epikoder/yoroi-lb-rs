use tokio;
use yoroi;

#[tokio::main]
async fn main() {
    let mut yr = yoroi::Yoroi::new("[::1]:8080".to_string());
    yr.registry().register_service(
        "kyc.Kyc".to_string(),
        "kyc".to_string(),
        vec!["localhost:50051".to_string()],
    );
    let err = yr.start_daemon().await.err();
    if let Some(err) = err {
        panic!("{}", err);
    };
}
