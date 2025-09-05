pub mod logging;
pub mod metrics;
pub mod rate_limit;

use async_trait::async_trait;
use std::net::SocketAddr;
use tracing::error;

// 临时使用基础类型，后续替换为hickory-dns的实际类型
pub type DnsMessage = Vec<u8>;

pub type MiddlewareResult = Result<Option<DnsMessage>, MiddlewareError>;

#[derive(Debug)]
pub enum MiddlewareError {
    RateLimited,
    Blocked,
    InternalError(String),
}

impl std::fmt::Display for MiddlewareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiddlewareError::RateLimited => write!(f, "Request rate limited"),
            MiddlewareError::Blocked => write!(f, "Request blocked by filter"),
            MiddlewareError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for MiddlewareError {}

/// 中间件trait - 处理DNS请求的中间件
#[async_trait]
pub trait Middleware: Send + Sync {
    /// 处理DNS请求，返回None表示继续处理，返回Some(Message)表示直接返回响应
    async fn handle_request(
        &self,
        request: &DnsMessage,
        client_addr: SocketAddr,
    ) -> MiddlewareResult;

    /// 处理DNS响应
    async fn handle_response(
        &self,
        request: &DnsMessage,
        response: &mut DnsMessage,
        client_addr: SocketAddr,
    ) -> Result<(), MiddlewareError>;

    /// 中间件名称
    fn name(&self) -> &str;
}

/// 中间件管道 - 按顺序执行所有中间件
pub struct MiddlewarePipeline {
    middlewares: Vec<Box<dyn Middleware>>,
}

impl MiddlewarePipeline {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add_middleware(&mut self, middleware: Box<dyn Middleware>) {
        self.middlewares.push(middleware);
    }

    /// 处理请求 - 如果任何中间件返回响应，则直接返回
    pub async fn handle_request(
        &self,
        request: &DnsMessage,
        client_addr: SocketAddr,
    ) -> MiddlewareResult {
        for middleware in &self.middlewares {
            match middleware.handle_request(request, client_addr).await {
                Ok(Some(response)) => return Ok(Some(response)),
                Ok(None) => continue,
                Err(e) => {
                    error!("中间件 {} 处理请求失败: {}", middleware.name(), e);
                    return Err(e);
                }
            }
        }
        Ok(None)
    }

    /// 处理响应 - 所有中间件都会处理响应
    pub async fn handle_response(
        &self,
        request: &DnsMessage,
        response: &mut DnsMessage,
        client_addr: SocketAddr,
    ) -> Result<(), MiddlewareError> {
        for middleware in &self.middlewares {
            if let Err(e) = middleware
                .handle_response(request, response, client_addr)
                .await
            {
                error!("中间件 {} 处理响应失败: {}", middleware.name(), e);
                return Err(e);
            }
        }
        Ok(())
    }
}

impl Default for MiddlewarePipeline {
    fn default() -> Self {
        Self::new()
    }
}
