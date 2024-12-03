use std::{collections::HashSet, error::Error};

use registry::ServiceRegistry;

mod registry;
pub mod server;
pub mod utils;

#[derive(Clone, Debug)]
pub struct MicroService {
    name: String,
    endpoints: HashSet<String>,
}

#[derive(Clone, Default)]
pub struct Yoroi {
    address: String,
    server: server::Server,
}

impl Yoroi {
    pub fn new(address: String) -> Self {
        Yoroi {
            address,
            server: server::Server::default(),
        }
    }

    pub fn registry(&mut self) -> &mut ServiceRegistry {
        &mut self.server.registry
    }

    pub fn handler(&self) -> server::ShutdownHandler {
        self.server.handler()
    }

    pub async fn start_daemon(&self) -> Result<(), Box<dyn Error>> {
        self.server.serve(self.address.as_str()).await
    }
}

#[cfg(test)]
mod tests_yoroi {
    use std::time::Duration;

    use crate::Yoroi;

    #[tokio::test]
    async fn test_daemon() {
        let yr = Yoroi::new("127.0.0.1:8000".to_string());
        let h = yr.handler();
        tokio::spawn(async move {
            let _ = yr.start_daemon().await;
        });
        h.graceful_shutdown(Some(Duration::from_secs(3))).await;
    }
}
