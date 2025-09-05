use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::fs;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub upstreams: Vec<UpstreamConfig>,
    pub cache: CacheConfig,
    pub filters: FilterConfig,
    pub middleware: MiddlewareConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: SocketAddr,
    pub tcp_enabled: bool,
    pub udp_enabled: bool,
    pub timeout: u64, // 超时时间(秒)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub name: String,
    pub addr: SocketAddr,
    pub protocol: Protocol,
    pub priority: u32,
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Protocol {
    UDP,
    TCP,
    DoT, // DNS over TLS
    DoH, // DNS over HTTPS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size: usize,
    pub ttl_min: u64,
    pub ttl_max: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub blocklist_enabled: bool,
    pub blocklist_files: Vec<String>,
    pub allowlist_enabled: bool,
    pub allowlist_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    pub logging_enabled: bool,
    pub metrics_enabled: bool,
    pub rate_limiting: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
}

impl Config {
    pub async fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("加载配置文件: {}", path);

        let content = fs::read_to_string(path).await?;
        let config: Config = serde_json::from_str(&content)?;

        info!("配置文件加载成功");
        Ok(config)
    }

    pub fn default_config() -> Self {
        Config {
            server: ServerConfig {
                listen_addr: "127.0.0.1:53".parse().unwrap(),
                tcp_enabled: true,
                udp_enabled: true,
                timeout: 5,
            },
            upstreams: vec![
                UpstreamConfig {
                    name: "Cloudflare".to_string(),
                    addr: "1.1.1.1:53".parse().unwrap(),
                    protocol: Protocol::UDP,
                    priority: 1,
                    timeout: 5,
                },
                UpstreamConfig {
                    name: "Google".to_string(),
                    addr: "8.8.8.8:53".parse().unwrap(),
                    protocol: Protocol::UDP,
                    priority: 2,
                    timeout: 5,
                },
            ],
            cache: CacheConfig {
                enabled: true,
                max_size: 10000,
                ttl_min: 60,
                ttl_max: 3600,
            },
            filters: FilterConfig {
                blocklist_enabled: false,
                blocklist_files: vec![],
                allowlist_enabled: false,
                allowlist_domains: vec![],
            },
            middleware: MiddlewareConfig {
                logging_enabled: true,
                metrics_enabled: true,
                rate_limiting: RateLimitConfig {
                    enabled: true,
                    requests_per_second: 100,
                    burst_size: 200,
                },
            },
        }
    }
}
