# DNS转发器 (DNS Forwarder)

一个用Rust编写的高性能DNS转发器，支持中间件处理、缓存、过滤和负载均衡。

## 架构特点

### 🔧 模块化设计

- **配置模块** (`src/config/`) - JSON配置文件支持
- **服务器模块** (`src/server/`) - 异步UDP/TCP DNS服务器
- **中间件模块** (`src/middleware/`) - 可插拔的请求处理中间件
- **解析器模块** (`src/resolver/`) - 上游DNS服务器连接和负载均衡
- **缓存模块** (`src/cache/`) - 智能DNS响应缓存
- **过滤器模块** (`src/filter/`) - 域名黑名单/白名单过滤
- **工具模块** (`src/utils/`) - 通用工具函数

### 🚀 技术栈

- **异步运行时**: Tokio
- **日志系统**: tracing + tracing-subscriber
- **序列化**: serde + serde_json
- **DNS协议**: hickory-dns
- **配置格式**: JSON

## 目录结构

```
dns/
├── Cargo.toml              # 项目依赖配置
├── src/
│   ├── main.rs            # 程序入口点
│   ├── config/            # 配置管理
│   │   └── mod.rs         # 配置结构和加载逻辑
│   ├── server/            # DNS服务器核心
│   │   └── mod.rs         # UDP/TCP服务器实现
│   ├── middleware/        # 中间件系统
│   │   ├── mod.rs         # 中间件框架和管道
│   │   ├── logging.rs     # 日志中间件
│   │   ├── rate_limit.rs  # 限流中间件
│   │   └── metrics.rs     # 指标收集中间件
│   ├── resolver/          # DNS解析器
│   │   └── mod.rs         # 上游服务器管理和查询
│   ├── cache/             # 缓存系统
│   │   └── mod.rs         # DNS响应缓存实现
│   ├── filter/            # 过滤系统
│   │   └── mod.rs         # 域名黑白名单过滤
│   └── utils/             # 实用工具
│       └── mod.rs         # DNS协议工具函数
├── config/                # 配置文件目录
│   └── config.json        # 主配置文件
├── tests/                 # 测试目录
└── test_client.rs         # 简单的DNS测试客户端
```

## 🔨 中间件架构

### 中间件管道

DNS转发器采用洋葱圈模型的中间件架构：

```
请求 → [日志] → [限流] → [指标] → [过滤] → [缓存] → [解析器] → 响应
      ↓                                                            ↑
   中间件1                                                    中间件N
```

### 内置中间件

1. **日志中间件** (`LoggingMiddleware`)
   - 记录所有DNS请求和响应
   - 支持结构化日志输出

2. **限流中间件** (`RateLimitMiddleware`)
   - 基于客户端IP的令牌桶限流
   - 可配置每秒请求数和突发容量

3. **指标中间件** (`MetricsMiddleware`)
   - 实时统计请求数、响应数、错误数
   - 定期输出统计报告

### 中间件接口

```rust
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn handle_request(
        &self,
        request: &DnsMessage,
        client_addr: SocketAddr,
    ) -> MiddlewareResult;

    async fn handle_response(
        &self,
        request: &DnsMessage,
        response: &mut DnsMessage,
        client_addr: SocketAddr,
    ) -> Result<(), MiddlewareError>;

    fn name(&self) -> &str;
}
```

## ⚙️ 配置系统

### 配置文件结构 (`config/config.json`)

```json
{
  "server": {
    "listen_addr": "127.0.0.1:8853",
    "tcp_enabled": true,
    "udp_enabled": true,
    "timeout": 5
  },
  "upstreams": [
    {
      "name": "Cloudflare Primary",
      "addr": "1.1.1.1:53",
      "protocol": "UDP",
      "priority": 1,
      "timeout": 5
    }
  ],
  "cache": {
    "enabled": true,
    "max_size": 10000,
    "ttl_min": 60,
    "ttl_max": 3600
  },
  "filters": {
    "blocklist_enabled": false,
    "blocklist_files": ["config/blocklist.txt"],
    "allowlist_enabled": false,
    "allowlist_domains": ["example.com"]
  },
  "middleware": {
    "logging_enabled": true,
    "metrics_enabled": true,
    "rate_limiting": {
      "enabled": true,
      "requests_per_second": 100,
      "burst_size": 200
    }
  }
}
```

## 🚀 快速开始

### 1. 编译和运行

```bash
# 克隆项目
git clone <repository>
cd dns

# 编译项目
cargo build --release

# 运行DNS服务器
cargo run
```

### 2. 测试DNS服务器

```bash
# 使用dig测试
dig @127.0.0.1 -p 8853 google.com

# 使用nslookup测试
nslookup google.com 127.0.0.1:8853

# 编译并运行测试客户端
rustc test_client.rs -o test_client
./test_client
```

## 📈 性能特性

### 异步处理

- 基于Tokio的完全异步实现
- 每个DNS查询在独立的任务中处理
- 支持高并发连接

### 智能缓存

- TTL感知的缓存系统
- 可配置的缓存大小和过期时间
- 自动清理过期条目

### 负载均衡

- 多上游服务器支持
- 优先级和故障转移
- 超时和重试机制

## 🔧 扩展性

### 添加新中间件

```rust
use async_trait::async_trait;
use crate::middleware::{Middleware, MiddlewareResult};

pub struct CustomMiddleware {
    // 中间件状态
}

#[async_trait]
impl Middleware for CustomMiddleware {
    async fn handle_request(&self, request: &DnsMessage, client_addr: SocketAddr) -> MiddlewareResult {
        // 处理请求逻辑
        Ok(None) // 继续处理
    }

    async fn handle_response(&self, request: &DnsMessage, response: &mut DnsMessage, client_addr: SocketAddr) -> Result<(), MiddlewareError> {
        // 处理响应逻辑
        Ok(())
    }

    fn name(&self) -> &str {
        "CustomMiddleware"
    }
}
```

### 添加新的上游协议

当前支持：

- UDP (标准DNS)
- TCP (TCP DNS)

计划支持：

- DoT (DNS over TLS)
- DoH (DNS over HTTPS)

## 📝 日志输出

DNS转发器使用结构化日志，示例输出：

```
2025-09-05T06:34:22.644215Z  INFO src\main.rs:23: 启动DNS转发器...
2025-09-05T06:34:22.654534Z  INFO src\server\mod.rs:72: DNS服务器初始化完成
2025-09-05T06:34:22.654661Z  INFO src\server\mod.rs:73: 监听地址: 127.0.0.1:8853
2025-09-05T06:34:22.655023Z  INFO src\server\mod.rs:76: 缓存启用: true
2025-09-05T06:34:22.655149Z  INFO src\server\mod.rs:77: 上游服务器数量: 4
```

## 🔒 安全特性

### 过滤系统

- 域名黑名单支持
- 域名白名单支持
- 支持多种格式的hosts文件

### 限流保护

- 基于IP的请求限流
- 令牌桶算法实现
- 防止DDoS攻击

## 🛠️ 开发状态

### ✅ 已实现功能

- [x] 基础DNS服务器 (UDP)
- [x] 配置系统 (JSON)
- [x] 中间件框架
- [x] 日志中间件
- [x] 限流中间件
- [x] 指标收集中间件
- [x] 缓存系统框架
- [x] 过滤器框架
- [x] 上游解析器框架

### 🚧 待完善功能

- [ ] 完整的DNS解析逻辑
- [ ] TCP DNS服务器
- [ ] DNS over TLS (DoT)
- [ ] DNS over HTTPS (DoH)
- [ ] 完整的缓存实现
- [ ] 域名过滤实现
- [ ] 指标API接口
- [ ] 配置热重载

## 📄 许可证

本项目采用 MIT 许可证。

## 🤝 贡献

欢迎提交Issue和Pull Request来改进这个项目！

## 📞 联系

如有问题或建议，请创建GitHub Issue。
