use std::net::SocketAddr;

pub struct Config {
    pub server: ServerConfig,
    pub cache: CacheConfig,
    pub upstreams: Vec<UpstreamConfig>,
    pub middleware: MiddlewareConfig,
}

pub struct ServerConfig {
    pub listen_addr: SocketAddr,
}
