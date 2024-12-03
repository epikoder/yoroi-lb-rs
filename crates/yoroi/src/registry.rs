use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use crate::MicroService;

#[derive(Clone, Default)]
pub struct ServiceRegistry {
    services: Arc<Mutex<HashMap<String, MicroService>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            services: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_service(&self, id: String, name: String, endpoints: Vec<String>) {
        let mut services = self.services.lock().unwrap();
        let service = services.entry(id.clone()).or_insert(MicroService {
            name,
            endpoints: HashSet::new(),
        });
        for endpoint in endpoints {
            service.endpoints.insert(endpoint);
        }
    }

    pub fn deregister_service(&self, id: &str) {
        let mut services = self.services.lock().unwrap();
        services.remove(id);
    }

    pub fn add_endpoint(&self, id: &str, endpoint: String) {
        let mut services = self.services.lock().unwrap();
        if let Some(service) = services.get_mut(id) {
            service.endpoints.insert(endpoint);
        }
    }

    pub fn get_service(&self, id: &str) -> Option<MicroService> {
        let services = self.services.lock().unwrap();
        services.get(id).cloned()
    }

    pub fn get_all_services(&self) -> HashMap<String, MicroService> {
        let services = self.services.lock().unwrap();
        services.clone()
    }

    pub fn resolve_endpoint(&self, id: String) -> Option<String> {
        let services = self.services.lock().unwrap();
        if let Some(service) = services.get(id.as_str()) {
            service.endpoints.clone().into_iter().next()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests_registry {
    use crate::ServiceRegistry;

    #[test]
    fn test_new() {
        ServiceRegistry::default();
    }

    #[test]
    fn test_add_service() {
        let srg = ServiceRegistry::default();
        srg.register_service(
            "ping-pong".to_string(),
            "Ping".to_string(),
            vec!["endpoint".to_string()],
        );
        assert_eq!(srg.get_all_services().len(), 1);
    }
}
