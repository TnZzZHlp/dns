use super::{Middleware, MiddlewareError, MiddlewareResult, DnsMessage};
use async_trait::async_trait;
use std::net::SocketAddr;
use tracing::{info, debug};

/// 日志中间件 - 记录所有DNS请求和响应
pub struct LoggingMiddleware {
    enabled: bool,
}

impl LoggingMiddleware {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn handle_request(
        &self,
        request: &DnsMessage,
        client_addr: SocketAddr,
    ) -> MiddlewareResult {
        if self.enabled {
            info!("DNS请求来自: {}, 大小: {} bytes", client_addr, request.len());
            debug!("请求内容: {:?}", request);
        }
        Ok(None) // 继续处理，不直接返回响应
    }

    async fn handle_response(
        &self,
        _request: &DnsMessage,
        response: &mut DnsMessage,
        client_addr: SocketAddr,
    ) -> Result<(), MiddlewareError> {
        if self.enabled {
            info!("DNS响应发送给: {}, 大小: {} bytes", client_addr, response.len());
            debug!("响应内容: {:?}", response);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "LoggingMiddleware"
    }
}
