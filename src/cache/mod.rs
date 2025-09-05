use crate::config::CacheConfig;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

pub type DnsMessage = Vec<u8>;

/// DNS缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    response: DnsMessage,
    created_at: Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn new(response: DnsMessage, ttl: Duration) -> Self {
        Self {
            response,
            created_at: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// DNS缓存管理器
pub struct DnsCache {
    enabled: bool,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    min_ttl: Duration,
    max_ttl: Duration,
}

impl DnsCache {
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            enabled: config.enabled,
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size: config.max_size,
            min_ttl: Duration::from_secs(config.ttl_min),
            max_ttl: Duration::from_secs(config.ttl_max),
        }
    }

    /// 生成缓存键
    fn generate_cache_key(&self, query: &DnsMessage) -> String {
        // 简单的哈希实现，实际应该解析DNS查询来生成更准确的键
        format!("{:x}", md5::compute(query))
    }

    /// 从缓存获取响应
    pub async fn get(&self, query: &DnsMessage) -> Option<DnsMessage> {
        if !self.enabled {
            return None;
        }

        let key = self.generate_cache_key(query);
        let cache = self.cache.read().await;

        if let Some(entry) = cache.get(&key) {
            if !entry.is_expired() {
                debug!("缓存命中: {}", key);
                return Some(entry.response.clone());
            } else {
                debug!("缓存过期: {}", key);
            }
        }

        debug!("缓存未命中: {}", key);
        None
    }

    /// 将响应存入缓存
    pub async fn put(&self, query: &DnsMessage, response: DnsMessage, ttl_hint: Option<u32>) {
        if !self.enabled {
            return;
        }

        let key = self.generate_cache_key(query);
        let mut cache = self.cache.write().await;

        // 如果缓存已满，清理过期条目或删除最旧的条目
        if cache.len() >= self.max_size {
            self.cleanup_cache(&mut cache).await;

            // 如果清理后仍然满了，删除一个条目
            if cache.len() >= self.max_size {
                if let Some(oldest_key) = cache.keys().next().cloned() {
                    cache.remove(&oldest_key);
                }
            }
        }

        // 计算TTL
        let ttl = if let Some(hint) = ttl_hint {
            let ttl_duration = Duration::from_secs(hint as u64);
            // 确保TTL在允许范围内
            ttl_duration.max(self.min_ttl).min(self.max_ttl)
        } else {
            self.min_ttl
        };

        let entry = CacheEntry::new(response, ttl);
        cache.insert(key.clone(), entry);

        debug!("缓存存储: {}, TTL: {:?}", key, ttl);
    }

    /// 清理过期的缓存条目
    async fn cleanup_cache(&self, cache: &mut HashMap<String, CacheEntry>) {
        let mut expired_keys = Vec::new();

        for (key, entry) in cache.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        for key in expired_keys {
            cache.remove(&key);
            debug!("清理过期缓存: {}", key);
        }
    }

    /// 手动清理所有过期条目
    pub async fn cleanup(&self) {
        if !self.enabled {
            return;
        }

        let mut cache = self.cache.write().await;
        let initial_size = cache.len();
        self.cleanup_cache(&mut cache).await;
        let final_size = cache.len();

        if initial_size != final_size {
            info!("缓存清理完成: {} -> {} 条目", initial_size, final_size);
        }
    }

    /// 清空所有缓存
    pub async fn clear(&self) {
        if !self.enabled {
            return;
        }

        let mut cache = self.cache.write().await;
        cache.clear();
        info!("缓存已清空");
    }

    /// 获取缓存统计信息
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let expired_entries = cache.values().filter(|entry| entry.is_expired()).count();

        CacheStats {
            total_entries,
            expired_entries,
            active_entries: total_entries - expired_entries,
            max_size: self.max_size,
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
    pub max_size: usize,
}
