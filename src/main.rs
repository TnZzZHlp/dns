mod cache;
mod config;
mod filter;
mod middleware;
mod resolver;
mod server;
mod utils;

use config::Config;
use server::DnsServer;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 tracing 日志系统
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("启动DNS转发器...");

    // 加载配置
    let config = match Config::load("config/config.json").await {
        Ok(config) => config,
        Err(e) => {
            error!("加载配置文件失败: {}", e);
            return Err(e);
        }
    };

    // 创建并启动DNS服务器
    let server = match DnsServer::new(config).await {
        Ok(server) => server,
        Err(e) => {
            error!("创建DNS服务器失败: {}", e);
            return Err(Box::new(e));
        }
    };

    info!("DNS转发器已启动");
    info!("监听地址: {}", server.listen_address());

    // 启动服务器
    if let Err(e) = server.run().await {
        error!("DNS服务器运行错误: {}", e);
        return Err(Box::new(e));
    }
    Ok(())
}
