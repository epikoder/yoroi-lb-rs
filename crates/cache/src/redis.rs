use super::Cache;
use redis::{Client, Commands, PubSubCommands};
use std::{fmt::Debug, str::FromStr};

#[derive(Clone)]
pub struct RedisCache {
    inner: Client,
}

impl RedisCache {
    pub fn new(address: String) -> Self {
        let client = match redis::Client::open(address) {
            Ok(c) => c,
            Err(e) => panic!("Failed to initailize cache:redis, err={}", e.to_string()),
        };
        RedisCache { inner: client }
    }
}

impl Cache for RedisCache {
    type Err = String;

    fn set<T>(&self, key: String, value: T) -> Result<(), Self::Err>
    where
        T: ToString,
    {
        let mut conn = match self.inner.get_connection().map_err(|e| e.to_string()) {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        if let Some(err) = conn.set::<_, _, ()>(key, value.to_string()).err() {
            let e = if let Some(e) = err.detail() {
                e.to_string()
            } else {
                err.code().unwrap().to_string()
            };
            return Err(e);
        }
        Ok(())
    }

    fn get<V>(&self, key: String) -> Result<Option<V>, Self::Err>
    where
        V: FromStr + Debug,
        <V as FromStr>::Err: std::fmt::Display,
    {
        let mut conn = match self.inner.get_connection().map_err(|e| e.to_string()) {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        let v: redis::Value = conn.get(key).unwrap();
        let v = match v {
            redis::Value::Nil => None,
            redis::Value::Int(i) => match V::from_str(i.to_string().as_str()) {
                Ok(v) => Some(v),
                Err(e) => return Err(e.to_string()),
            },
            redis::Value::Data(v) => {
                let v = match String::from_utf8(v) {
                    Ok(v) => match V::from_str(v.as_str()) {
                        Ok(v) => Some(v),
                        Err(e) => return Err(e.to_string()),
                    },
                    Err(e) => return Err(e.to_string()),
                };
                v
            }
            redis::Value::Bulk(_) => return Err("result is bulk".to_string()),
            redis::Value::Status(_) => return Ok(None),
            redis::Value::Okay => return Ok(None),
        };
        Ok(v)
    }

    fn forget<V>(&self, key: String) -> Result<Option<V>, Self::Err>
    where
        V: FromStr,
        <V as FromStr>::Err: std::fmt::Display,
    {
        let mut conn = match self.inner.get_connection().map_err(|e| e.to_string()) {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        let v: redis::Value = conn.get_del(key).unwrap();
        let v = match v {
            redis::Value::Nil => None,
            redis::Value::Int(i) => match V::from_str(i.to_string().as_str()) {
                Ok(v) => Some(v),
                Err(e) => return Err(e.to_string()),
            },
            redis::Value::Data(v) => {
                let v = match String::from_utf8(v) {
                    Ok(v) => match V::from_str(v.as_str()) {
                        Ok(v) => Some(v),
                        Err(e) => return Err(e.to_string()),
                    },
                    Err(e) => return Err(e.to_string()),
                };
                v
            }
            redis::Value::Bulk(_) => return Err("result is bulk".to_string()),
            redis::Value::Status(_) => return Ok(None),
            redis::Value::Okay => return Ok(None),
        };
        Ok(v)
    }

    fn subscribe<V, F>(&self, topic: String, mut f: F) -> Result<Option<V>, Self::Err>
    where
        V: FromStr + Clone,
        <V as FromStr>::Err: std::fmt::Debug,
        F: FnMut(V) -> super::ControlFlow<V>,
    {
        let mut conn = match self.inner.get_connection().map_err(|e| e.to_string()) {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        let r = conn.subscribe(&[&topic], |msg| {
            let payload = msg.get_payload::<String>().unwrap();
            let msg = V::from_str(&payload).unwrap();
            match f(msg) {
                super::ControlFlow::Break(v) => redis::ControlFlow::Break(v),
                super::ControlFlow::Continue => redis::ControlFlow::Continue,
            }
        });
        match r {
            Ok(v) => Ok(Some(v)),
            Err(e) => Err(e.to_string()),
        }
    }

    fn publish<V: ToString>(&self, topic: String, v: V) -> Result<(), Self::Err> {
        let mut conn = match self.inner.get_connection().map_err(|e| e.to_string()) {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        let rv: redis::Value = match conn.publish(topic, v.to_string()) {
            Ok(rv) => rv,
            Err(e) => return Err(e.to_string()),
        };
        let rv = match rv {
            redis::Value::Nil => None,
            redis::Value::Int(i) => Some(i.to_string()),
            redis::Value::Data(v) => match String::from_utf8(v) {
                Ok(v) => Some(v),
                Err(e) => return Err(e.to_string()),
            },
            redis::Value::Bulk(_) => return Err("result is bulk".to_string()),
            redis::Value::Status(s) => return Err(s),
            redis::Value::Okay => None,
        };
        if let Some(_) = rv {
            Ok(())
        } else {
            Err("Something went wrong".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_new_redis_cache() {
        let _ = RedisCache::new("redis://127.0.0.1:6379".to_string());
    }

    #[tokio::test]
    async fn test_set_get() {
        let cache = RedisCache::new("redis://127.0.0.1:6379".to_string());

        // test int
        cache.set("key.name".to_string(), 1).unwrap();
        let v = cache.get::<u8>("key.name".to_string()).unwrap().unwrap();
        assert_eq!(v, 1);

        // test string
        cache
            .set("key.name".to_string(), String::from("hello world"))
            .unwrap();
        let v = cache
            .get::<String>("key.name".to_string())
            .unwrap()
            .unwrap();
        assert_eq!(v, String::from("hello world"));

        // test 'static str
        cache.set("key.name".to_string(), "hello world").unwrap();
        let v = cache
            .get::<String>("key.name".to_string())
            .unwrap()
            .unwrap();
        assert_eq!(v, "hello world");

        // test [u8]
        cache
            .set(
                "key.name".to_string(),
                String::from_utf8(b"bytes".to_vec()).unwrap(),
            )
            .unwrap();
        let v = cache.get::<String>("key.name".to_string()).unwrap();
        if let Some(v) = v {
            assert_eq!(v, "bytes".to_string());
        } else {
            assert!(false);
        }
    }

    #[tokio::test]
    async fn test_publish() {
        let cache = RedisCache::new("redis://127.0.0.1:6379".to_string());
        cache
            .publish("channel1".to_string(), "heelo")
            .unwrap_or_else(|_| {});
    }

    #[tokio::test]
    async fn test_subscribe() {
        let cache = RedisCache::new("redis://127.0.0.1:6379".to_string());

        let cache_clone = cache.clone();
        let handler = std::thread::spawn(move || {
            cache.subscribe::<String, _>(
                "channel1".to_string(),
                |msg| -> crate::ControlFlow<String> { crate::ControlFlow::Break(msg) },
            )
        });
        sleep(Duration::from_secs(2)).await;
        let _ = cache_clone.publish(
            "channel1".to_string(),
            "Hey! check out what I sent ya way :)",
        );
        let _ = handler.join().unwrap();
    }
}
