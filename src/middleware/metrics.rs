use super::{Middleware, MiddlewareError, MiddlewareResult, DnsMessage};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;

/// 指标收集中间件 - 收集DNS服务器的统计信息
pub struct MetricsMiddleware {
    enabled: bool,
    total_requests: Arc<AtomicU64>,
    total_responses: Arc<AtomicU64>,
    blocked_requests: Arc<AtomicU64>,
    rate_limited_requests: Arc<AtomicU64>,
}

impl MetricsMiddleware {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            total_requests: Arc::new(AtomicU64::new(0)),
            total_responses: Arc::new(AtomicU64::new(0)),
            blocked_requests: Arc::new(AtomicU64::new(0)),
            rate_limited_requests: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn get_metrics(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_responses: self.total_responses.load(Ordering::Relaxed),
            blocked_requests: self.blocked_requests.load(Ordering::Relaxed),
            rate_limited_requests: self.rate_limited_requests.load(Ordering::Relaxed),
        }
    }

    pub fn print_metrics(&self) {
        let metrics = self.get_metrics();
        info!("=== DNS服务器指标 ===");
        info!("总请求数: {}", metrics.total_requests);
        info!("总响应数: {}", metrics.total_responses);
        info!("被阻止请求数: {}", metrics.blocked_requests);
        info!("被限流请求数: {}", metrics.rate_limited_requests);
        info!("==================");
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub total_responses: u64,
    pub blocked_requests: u64,
    pub rate_limited_requests: u64,
}

#[async_trait]
impl Middleware for MetricsMiddleware {
    async fn handle_request(
        &self,
        _request: &DnsMessage,
        _client_addr: SocketAddr,
    ) -> MiddlewareResult {
        if self.enabled {
            self.total_requests.fetch_add(1, Ordering::Relaxed);
        }
        Ok(None) // 继续处理
    }

    async fn handle_response(
        &self,
        _request: &DnsMessage,
        _response: &mut DnsMessage,
        _client_addr: SocketAddr,
    ) -> Result<(), MiddlewareError> {
        if self.enabled {
            self.total_responses.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "MetricsMiddleware"
    }
}
