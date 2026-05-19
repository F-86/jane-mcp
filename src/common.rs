use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::find_config_path()?;
        info!("Loading config from: {}", config_path.display());
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
        Ok(config)
    }

    fn find_config_path() -> Result<PathBuf> {
        let candidates = [
            PathBuf::from("config/config.toml"),
            PathBuf::from("/etc/jane-mcp/config.toml"),
        ];
        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }
        let first = candidates.first().unwrap();
        anyhow::bail!(
            "Config file not found. Searched: {:?}. Please copy config.toml.example to {} and fill in your settings.",
            candidates,
            first.display()
        )
    }
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self { client })
    }

    pub fn inner(&self) -> &Client {
        &self.client
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client")
    }
}

#[derive(Clone)]
pub struct TtlCache<V: Clone> {
    store: Arc<RwLock<HashMap<String, (V, Instant)>>>,
    ttl: Duration,
}

impl<V: Clone> TtlCache<V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    pub async fn get(&self, key: &str) -> Option<V> {
        let store = self.store.read().await;
        if let Some((value, instant)) = store.get(key) {
            if instant.elapsed() < self.ttl {
                return Some(value.clone());
            }
        }
        None
    }

    pub async fn set(&self, key: String, value: V) {
        let mut store = self.store.write().await;
        store.insert(key, (value, Instant::now()));
    }

    pub async fn cleanup(&self) {
        let mut store = self.store.write().await;
        let now = Instant::now();
        store.retain(|_, (_, instant)| now.duration_since(*instant) < self.ttl);
    }
}

pub fn init_logging() {
    let _subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .json()
        .init();
}
