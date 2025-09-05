use super::{Middleware, MiddlewareError, MiddlewareResult, DnsMessage};
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::warn;

/// 限流桶
#[derive(Debug)]
struct RateLimitBucket {
    tokens: u32,
    last_refill: Instant,
    max_tokens: u32,
    refill_rate: u32, // tokens per second
}

impl RateLimitBucket {
    fn new(max_tokens: u32, refill_rate: u32) -> Self {
        Self {
            tokens: max_tokens,
            last_refill: Instant::now(),
            max_tokens,
            refill_rate,
        }
    }

    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate as f64) as u32;
        
        if tokens_to_add > 0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.max_tokens);
            self.last_refill = now;
        }
    }
}

/// 限流中间件 - 基于客户端IP进行限流
pub struct RateLimitMiddleware {
    enabled: bool,
    buckets: Arc<Mutex<HashMap<SocketAddr, RateLimitBucket>>>,
    max_tokens: u32,
    refill_rate: u32,
}

impl RateLimitMiddleware {
    pub fn new(enabled: bool, requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            enabled,
            buckets: Arc::new(Mutex::new(HashMap::new())),
            max_tokens: burst_size,
            refill_rate: requests_per_second,
        }
    }

    async fn check_rate_limit(&self, client_addr: SocketAddr) -> bool {
        if !self.enabled {
            return true;
        }

        let mut buckets = self.buckets.lock().await;
        let bucket = buckets
            .entry(client_addr)
            .or_insert_with(|| RateLimitBucket::new(self.max_tokens, self.refill_rate));
        
        bucket.try_consume()
    }
}

#[async_trait]
impl Middleware for RateLimitMiddleware {
    async fn handle_request(
        &self,
        _request: &DnsMessage,
        client_addr: SocketAddr,
    ) -> MiddlewareResult {
        if !self.check_rate_limit(client_addr).await {
            warn!("客户端 {} 请求被限流", client_addr);
            return Err(MiddlewareError::RateLimited);
        }
        Ok(None) // 继续处理
    }

    async fn handle_response(
        &self,
        _request: &DnsMessage,
        _response: &mut DnsMessage,
        _client_addr: SocketAddr,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "RateLimitMiddleware"
    }
}
