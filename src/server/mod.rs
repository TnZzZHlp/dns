use crate::cache::DnsCache;
use crate::config::Config;
use crate::middleware::MiddlewarePipeline;
use crate::middleware::logging::LoggingMiddleware;
use crate::middleware::metrics::MetricsMiddleware;
use crate::middleware::rate_limit::RateLimitMiddleware;
use crate::resolver::DnsResolver;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// DNS转发服务器
pub struct DnsServer {
    config: Config,
    middleware_pipeline: Arc<MiddlewarePipeline>,
    resolver: Arc<Mutex<DnsResolver>>,
    cache: Arc<DnsCache>,
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

        info!("DNS服务器初始化完成");
        info!("监听地址: {}", config.server.listen_addr);
        info!("UDP启用: {}", config.server.udp_enabled);
        info!("TCP启用: {}", config.server.tcp_enabled);
        info!("缓存启用: {}", config.cache.enabled);
        info!("上游服务器数量: {}", config.upstreams.len());

        Ok(Self {
            config,
            middleware_pipeline: Arc::new(middleware_pipeline),
            resolver,
            cache,
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
            let socket = Arc::new(socket);
            let resolver = self.resolver.clone();
            let cache = self.cache.clone();
            let pipeline = self.middleware_pipeline.clone();

            // 主循环: 克隆引用供 move 使用
            let config_cache_enabled = self.config.cache.enabled;

            loop {
                let mut buffer = vec![0u8; 1500]; // 以太网MTU上限, 兼容 EDNS(不拆分)
                let (len, client_addr) = match socket.recv_from(&mut buffer).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("UDP接收错误: {}", e);
                        continue;
                    }
                };
                buffer.truncate(len);
                let query = buffer;

                let socket = socket.clone();
                let resolver = resolver.clone();
                let cache = cache.clone();
                let pipeline = pipeline.clone();
                // metrics 中间件统计通过中间件本身进行，这里不再手动计数

                tokio::spawn(async move {
                    // 中间件请求阶段
                    match pipeline.handle_request(&query, client_addr).await {
                        Ok(Some(short_circuit)) => {
                            let _ = socket.send_to(&short_circuit, client_addr).await;
                            return;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            debug!("请求被中间件拒绝: {}", e);
                            return;
                        }
                    }

                    // 缓存查找
                    if config_cache_enabled && let Some(cached) = cache.get(&query).await {
                        debug!("命中缓存, 直接返回");
                        let mut resp = cached.clone();
                        if let Err(e) = pipeline
                            .handle_response(&query, &mut resp, client_addr)
                            .await
                        {
                            debug!("响应中间件处理缓存失败: {}", e);
                        }
                        let _ = socket.send_to(&resp, client_addr).await;
                        return;
                    }

                    // 上游解析
                    let upstream_resp = {
                        let mut resolver = resolver.lock().await;
                        resolver.resolve(&query).await
                    };

                    let mut response = match upstream_resp {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("上游解析失败: {}", e);
                            return;
                        }
                    };

                    // 写入缓存
                    if config_cache_enabled {
                        cache.put(&query, response.clone(), None).await;
                    }

                    // 响应中间件
                    if let Err(e) = pipeline
                        .handle_response(&query, &mut response, client_addr)
                        .await
                    {
                        debug!("响应中间件处理失败: {}", e);
                    }

                    if let Err(e) = socket.send_to(&response, client_addr).await {
                        error!("发送响应失败: {}", e);
                    }
                });
            }
        }

        // 启动TCP服务器（如果需要）
        if self.config.server.tcp_enabled {
            // TODO: 实现TCP服务器
            warn!("TCP服务器功能待实现");
        }

        // 保持主线程运行
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| DnsServerError::RuntimeError(format!("等待信号失败: {}", e)))?;

        info!("收到停止信号，正在关闭DNS服务器...");
        Ok(())
    }
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
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
