use crate::config::FilterConfig;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{info, error, debug};

pub type DnsMessage = Vec<u8>;

/// DNS过滤器 - 支持域名黑名单和白名单
pub struct DnsFilter {
    blocklist_enabled: bool,
    allowlist_enabled: bool,
    blocked_domains: Arc<RwLock<HashSet<String>>>,
    allowed_domains: Arc<RwLock<HashSet<String>>>,
}

impl DnsFilter {
    pub async fn new(config: &FilterConfig) -> Result<Self, FilterError> {
        let filter = Self {
            blocklist_enabled: config.blocklist_enabled,
            allowlist_enabled: config.allowlist_enabled,
            blocked_domains: Arc::new(RwLock::new(HashSet::new())),
            allowed_domains: Arc::new(RwLock::new(HashSet::new())),
        };

        // 加载黑名单文件
        if config.blocklist_enabled {
            for file_path in &config.blocklist_files {
                if let Err(e) = filter.load_blocklist_file(file_path).await {
                    error!("加载黑名单文件 {} 失败: {}", file_path, e);
                }
            }
        }

        // 加载白名单
        if config.allowlist_enabled {
            let mut allowed = filter.allowed_domains.write().await;
            for domain in &config.allowlist_domains {
                allowed.insert(domain.to_lowercase());
            }
            info!("加载了 {} 个白名单域名", config.allowlist_domains.len());
        }

        Ok(filter)
    }

    /// 检查域名是否应该被阻止
    pub async fn should_block(&self, _query: &DnsMessage) -> bool {
        // TODO: 实际解析DNS查询包来提取域名
        debug!("过滤器检查功能待实现");
        false
    }

    /// 获取过滤器统计信息
    pub async fn stats(&self) -> FilterStats {
        let blocked_count = if self.blocklist_enabled {
            self.blocked_domains.read().await.len()
        } else {
            0
        };

        let allowed_count = if self.allowlist_enabled {
            self.allowed_domains.read().await.len()
        } else {
            0
        };

        FilterStats {
            blocklist_enabled: self.blocklist_enabled,
            allowlist_enabled: self.allowlist_enabled,
            blocked_domains_count: blocked_count,
            allowed_domains_count: allowed_count,
        }
    }

    async fn load_blocklist_file(&self, file_path: &str) -> Result<(), FilterError> {
        info!("加载黑名单文件: {}", file_path);
        
        let content = fs::read_to_string(file_path).await
            .map_err(|e| FilterError::FileError(e.to_string()))?;

        let mut blocked = self.blocked_domains.write().await;
        let mut count = 0;

        for line in content.lines() {
            let line = line.trim();
            
            // 跳过空行和注释
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            blocked.insert(line.to_lowercase());
            count += 1;
        }

        info!("从文件 {} 加载了 {} 个黑名单域名", file_path, count);
        Ok(())
    }
}

#[derive(Debug)]
pub struct FilterStats {
    pub blocklist_enabled: bool,
    pub allowlist_enabled: bool,
    pub blocked_domains_count: usize,
    pub allowed_domains_count: usize,
}

#[derive(Debug)]
pub enum FilterError {
    FileError(String),
}

impl std::fmt::Display for FilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterError::FileError(msg) => write!(f, "文件错误: {}", msg),
        }
    }
}

impl std::error::Error for FilterError {}

