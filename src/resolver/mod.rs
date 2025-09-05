use crate::config::{UpstreamConfig, Protocol};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use tracing::{info, error, warn};

pub type DnsMessage = Vec<u8>;

/// DNS解析器 - 负责将请求转发到上游DNS服务器
pub struct DnsResolver {
    upstreams: Vec<UpstreamConfig>,
    current_upstream: usize,
}

impl DnsResolver {
    pub fn new(upstreams: Vec<UpstreamConfig>) -> Self {
        Self {
            upstreams,
            current_upstream: 0,
        }
    }

    /// 解析DNS查询
    pub async fn resolve(&mut self, query: &DnsMessage) -> Result<DnsMessage, ResolverError> {
        // 尝试所有上游服务器
        for _attempt in 0..self.upstreams.len() {
            let upstream = &self.upstreams[self.current_upstream];
            
            info!("尝试使用上游服务器: {}", upstream.name);
            
            match self.query_upstream(upstream, query).await {
                Ok(response) => {
                    info!("从上游服务器 {} 获得响应", upstream.name);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("上游服务器 {} 查询失败: {}", upstream.name, e);
                    // 切换到下一个上游服务器
                    self.current_upstream = (self.current_upstream + 1) % self.upstreams.len();
                }
            }
        }

        error!("所有上游服务器都不可用");
        Err(ResolverError::AllUpstreamsUnavailable)
    }

    /// 查询特定的上游服务器
    async fn query_upstream(
        &self,
        upstream: &UpstreamConfig,
        query: &DnsMessage,
    ) -> Result<DnsMessage, ResolverError> {
        match upstream.protocol {
            Protocol::UDP => self.query_udp(upstream, query).await,
            Protocol::TCP => self.query_tcp(upstream, query).await,
            Protocol::DoT => {
                // TODO: 实现 DNS over TLS
                error!("DNS over TLS 暂未实现");
                Err(ResolverError::UnsupportedProtocol)
            }
            Protocol::DoH => {
                // TODO: 实现 DNS over HTTPS
                error!("DNS over HTTPS 暂未实现");
                Err(ResolverError::UnsupportedProtocol)
            }
        }
    }

    /// UDP查询
    async fn query_udp(
        &self,
        upstream: &UpstreamConfig,
        query: &DnsMessage,
    ) -> Result<DnsMessage, ResolverError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await
            .map_err(|e| ResolverError::NetworkError(e.to_string()))?;

        // 发送查询
        socket.send_to(query, upstream.addr).await
            .map_err(|e| ResolverError::NetworkError(e.to_string()))?;

        // 接收响应
        let mut buffer = vec![0u8; 512]; // DNS UDP 最大包大小
        let result = timeout(
            Duration::from_secs(upstream.timeout),
            socket.recv_from(&mut buffer),
        ).await;

        match result {
            Ok(Ok((len, _addr))) => {
                buffer.truncate(len);
                Ok(buffer)
            }
            Ok(Err(e)) => Err(ResolverError::NetworkError(e.to_string())),
            Err(_) => Err(ResolverError::Timeout),
        }
    }

    /// TCP查询
    async fn query_tcp(
        &self,
        upstream: &UpstreamConfig,
        query: &DnsMessage,
    ) -> Result<DnsMessage, ResolverError> {
        use tokio::net::TcpStream;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut stream = timeout(
            Duration::from_secs(upstream.timeout),
            TcpStream::connect(upstream.addr),
        ).await
        .map_err(|_| ResolverError::Timeout)?
        .map_err(|e| ResolverError::NetworkError(e.to_string()))?;

        // TCP DNS消息需要长度前缀
        let mut message = Vec::new();
        message.extend_from_slice(&(query.len() as u16).to_be_bytes());
        message.extend_from_slice(query);

        // 发送查询
        stream.write_all(&message).await
            .map_err(|e| ResolverError::NetworkError(e.to_string()))?;

        // 读取响应长度
        let mut len_bytes = [0u8; 2];
        stream.read_exact(&mut len_bytes).await
            .map_err(|e| ResolverError::NetworkError(e.to_string()))?;
        let response_len = u16::from_be_bytes(len_bytes) as usize;

        // 读取响应数据
        let mut response = vec![0u8; response_len];
        stream.read_exact(&mut response).await
            .map_err(|e| ResolverError::NetworkError(e.to_string()))?;

        Ok(response)
    }

    /// 获取当前上游服务器信息
    pub fn current_upstream(&self) -> Option<&UpstreamConfig> {
        self.upstreams.get(self.current_upstream)
    }
}

#[derive(Debug)]
pub enum ResolverError {
    AllUpstreamsUnavailable,
    NetworkError(String),
    Timeout,
    UnsupportedProtocol,
}

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolverError::AllUpstreamsUnavailable => write!(f, "所有上游服务器不可用"),
            ResolverError::NetworkError(msg) => write!(f, "网络错误: {}", msg),
            ResolverError::Timeout => write!(f, "请求超时"),
            ResolverError::UnsupportedProtocol => write!(f, "不支持的协议"),
        }
    }
}

impl std::error::Error for ResolverError {}
