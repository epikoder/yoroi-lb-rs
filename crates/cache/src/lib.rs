use std::{fmt::Debug, str::FromStr};

pub mod redis;

pub trait Cache {
    type Err: Debug;

    fn set<V>(&self, key: String, value: V) -> Result<(), Self::Err>
    where
        V: ToString;
    fn get<V>(&self, key: String) -> Result<Option<V>, Self::Err>
    where
        V: FromStr + Debug,
        <V as FromStr>::Err: std::fmt::Display;
    fn forget<V>(&self, key: String) -> Result<Option<V>, Self::Err>
    where
        V: FromStr,
        <V as FromStr>::Err: std::fmt::Display;
    fn subscribe<V, F>(&self, topic: String, f: F) -> Result<Option<V>, Self::Err>
    where
        V: FromStr + Clone,
        <V as FromStr>::Err: std::fmt::Debug,
        F: FnMut(V) -> ControlFlow<V>;

    fn publish<V: ToString>(&self, topic: String, v: V) -> Result<(), Self::Err>;
}

pub enum ControlFlow<T> {
    Continue,
    Break(T),
}

/// Wrapper for struct implementing Cache 
pub struct CacheStorage<C: Cache> {
    inner: C,
}

impl<C: Cache> CacheStorage<C> {
    pub fn new(cache: C) -> Self {
        CacheStorage { inner: cache }
    }

    pub fn set<V>(&self, key: String, value: V) -> Result<(), C::Err>
    where
        V: ToString,
    {
        self.inner.set(key, value)
    }

    pub fn get<V>(&self, key: String) -> Result<Option<V>, C::Err>
    where
        V: FromStr + Debug,
        <V as FromStr>::Err: std::fmt::Display,
    {
        self.inner.get(key)
    }

    pub fn forget<V>(&self, key: String) -> Result<Option<V>, C::Err>
    where
        V: FromStr,
        <V as FromStr>::Err: std::fmt::Display,
    {
        self.inner.forget(key)
    }

    pub fn subscribe<V, F>(&self, topic: String, f: F) -> Result<Option<V>, C::Err>
    where
        V: FromStr + Clone,
        <V as FromStr>::Err: std::fmt::Debug,
        F: FnMut(V) -> ControlFlow<V>,
    {
        self.inner.subscribe(topic, f)
    }

    pub fn publish<V: ToString>(&self, topic: String, v: V) -> Result<(), C::Err> {
        self.inner.publish(topic, v)
    }
}
