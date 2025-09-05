use crate::config::Config;
use crate::middleware::MiddlewarePipeline;
use crate::middleware::logging::LoggingMiddleware;
use crate::middleware::rate_limit::RateLimitMiddleware;
use crate::middleware::metrics::MetricsMiddleware;
use crate::resolver::DnsResolver;
use crate::cache::DnsCache;
use crate::filter::DnsFilter;
use crate::utils::{extract_query_id, create_dns_error_response, dns_rcode};

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tracing::{info, error, warn, debug};

pub type DnsMessage = Vec<u8>;

/// DNS转发服务器
pub struct DnsServer {
    config: Config,
    middleware_pipeline: MiddlewarePipeline,
    resolver: Arc<Mutex<DnsResolver>>,
    cache: Arc<DnsCache>,
    filter: Arc<DnsFilter>,
    metrics: Arc<MetricsMiddleware>,
}

impl DnsServer {
    /// 创建新的DNS服务器实例
    pub async fn new(config: Config) -> Result<Self, DnsServerError> {
        // 创建中间件管道
        let mut middleware_pipeline = MiddlewarePipeline::new();
        
        // 添加日志中间件
        if config.middleware.logging_enabled {
            let logging_middleware = LoggingMiddleware::new(true);
            middleware_pipeline.add_middleware(Box::new(logging_middleware));
        }

        // 添加限流中间件
        if config.middleware.rate_limiting.enabled {
            let rate_limit_middleware = RateLimitMiddleware::new(
                true,
                config.middleware.rate_limiting.requests_per_second,
                config.middleware.rate_limiting.burst_size,
            );
            middleware_pipeline.add_middleware(Box::new(rate_limit_middleware));
        }

        // 创建指标中间件
        let metrics = Arc::new(MetricsMiddleware::new(config.middleware.metrics_enabled));
        if config.middleware.metrics_enabled {
            // 创建一个新的指标中间件实例来添加到管道
            let metrics_middleware = MetricsMiddleware::new(true);
            middleware_pipeline.add_middleware(Box::new(metrics_middleware));
        }

        // 创建解析器
        let resolver = Arc::new(Mutex::new(DnsResolver::new(config.upstreams.clone())));

        // 创建缓存
        let cache = Arc::new(DnsCache::new(&config.cache));

        // 创建过滤器
        let filter = Arc::new(
            DnsFilter::new(&config.filters)
                .await
                .map_err(|e| DnsServerError::InitializationError(e.to_string()))?,
        );

        info!("DNS服务器初始化完成");
        info!("监听地址: {}", config.server.listen_addr);
        info!("UDP启用: {}", config.server.udp_enabled);
        info!("TCP启用: {}", config.server.tcp_enabled);
        info!("缓存启用: {}", config.cache.enabled);
        info!("上游服务器数量: {}", config.upstreams.len());

        Ok(Self {
            config,
            middleware_pipeline,
            resolver,
            cache,
            filter,
            metrics,
        })
    }

    /// 获取监听地址
    pub fn listen_address(&self) -> SocketAddr {
        self.config.server.listen_addr
    }

    /// 启动DNS服务器
    pub async fn run(&self) -> Result<(), DnsServerError> {
        info!("启动DNS服务器在地址: {}", self.config.server.listen_addr);

        // 启动UDP服务器
        if self.config.server.udp_enabled {
            let listen_addr = self.config.server.listen_addr;
            
            info!("启动UDP服务器在地址: {}", listen_addr);
            
            let socket = UdpSocket::bind(listen_addr)
                .await
                .map_err(|e| DnsServerError::NetworkError(e.to_string()))?;

            let mut buffer = vec![0u8; 512];

            loop {
                match socket.recv_from(&mut buffer).await {
                    Ok((len, client_addr)) => {
                        let query = buffer[..len].to_vec();
                        debug!("收到来自 {} 的DNS查询，长度: {} bytes", client_addr, len);
                        
                        // 简单响应 - 返回服务器失败
                        if let Some(query_id) = extract_query_id(&query) {
                            let error_response = create_dns_error_response(query_id, dns_rcode::SERVER_FAILURE);
                            if let Err(e) = socket.send_to(&error_response, client_addr).await {
                                error!("发送响应失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("UDP接收错误: {}", e);
                    }
                }
            }
        }

        // 启动TCP服务器（如果需要）
        if self.config.server.tcp_enabled {
            // TODO: 实现TCP服务器
            warn!("TCP服务器功能待实现");
        }

        // 启动统计信息定时打印
        if self.config.middleware.metrics_enabled {
            let metrics = self.metrics.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    metrics.print_metrics();
                }
            });
        }

        // 保持主线程运行
        tokio::signal::ctrl_c().await.map_err(|e| {
            DnsServerError::RuntimeError(format!("等待信号失败: {}", e))
        })?;

        info!("收到停止信号，正在关闭DNS服务器...");
        Ok(())
    }
}

#[derive(Debug)]
pub enum DnsServerError {
    InitializationError(String),
    NetworkError(String),
    RuntimeError(String),
}

impl std::fmt::Display for DnsServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DnsServerError::InitializationError(msg) => write!(f, "初始化错误: {}", msg),
            DnsServerError::NetworkError(msg) => write!(f, "网络错误: {}", msg),
            DnsServerError::RuntimeError(msg) => write!(f, "运行时错误: {}", msg),
        }
    }
}

impl std::error::Error for DnsServerError {}
