/// 实用工具函数模块
use std::net::IpAddr;
use tracing::debug;

/// 解析IP地址字符串
pub fn parse_ip_address(addr_str: &str) -> Result<IpAddr, std::net::AddrParseError> {
    addr_str.parse()
}

/// 检查是否为有效的IPv4地址
pub fn is_ipv4(addr: &IpAddr) -> bool {
    matches!(addr, IpAddr::V4(_))
}

/// 检查是否为有效的IPv6地址
pub fn is_ipv6(addr: &IpAddr) -> bool {
    matches!(addr, IpAddr::V6(_))
}

/// 格式化字节大小
pub fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// 格式化持续时间
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let millis = duration.subsec_millis();

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else if seconds > 0 {
        format!("{}.{}s", seconds, millis / 100)
    } else {
        format!("{}ms", millis)
    }
}

/// 简单的DNS消息格式验证
pub fn is_valid_dns_message(data: &[u8]) -> bool {
    // DNS消息最小长度为12字节（头部）
    if data.len() < 12 {
        debug!("DNS消息长度不足: {} bytes", data.len());
        return false;
    }

    // 检查DNS头部格式的基本有效性
    let flags = u16::from_be_bytes([data[2], data[3]]);
    let qr_bit = (flags >> 15) & 1; // 查询/响应位
    let opcode = (flags >> 11) & 0x0F; // 操作码
    
    // 操作码应该为0（标准查询）
    if opcode != 0 {
        debug!("无效的DNS操作码: {}", opcode);
        return false;
    }

    debug!("DNS消息验证通过: {} bytes, QR={}", data.len(), qr_bit);
    true
}

/// 从DNS消息中提取查询ID
pub fn extract_query_id(data: &[u8]) -> Option<u16> {
    if data.len() >= 2 {
        Some(u16::from_be_bytes([data[0], data[1]]))
    } else {
        None
    }
}

/// 生成DNS响应的错误消息
pub fn create_dns_error_response(query_id: u16, error_code: u8) -> Vec<u8> {
    let mut response = vec![0u8; 12];
    
    // 设置查询ID
    response[0..2].copy_from_slice(&query_id.to_be_bytes());
    
    // 设置标志位：QR=1(响应), RA=1(递归可用), RCODE=error_code
    let flags = 0x8180 | (error_code as u16);
    response[2..4].copy_from_slice(&flags.to_be_bytes());
    
    // 其他字段保持为0
    response
}

/// DNS响应代码常量
pub mod dns_rcode {
    pub const NO_ERROR: u8 = 0;     // 无错误
    pub const FORMAT_ERROR: u8 = 1; // 格式错误
    pub const SERVER_FAILURE: u8 = 2; // 服务器失败
    pub const NAME_ERROR: u8 = 3;   // 域名不存在
    pub const NOT_IMPLEMENTED: u8 = 4; // 未实现
    pub const REFUSED: u8 = 5;      // 拒绝
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
    }

    #[test]
    fn test_format_duration() {
        use std::time::Duration;
        
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_secs(1)), "1.0s");
        assert_eq!(format_duration(Duration::from_secs(61)), "1m 1s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
    }

    #[test]
    fn test_dns_message_validation() {
        // 有效的DNS消息头部
        let valid_dns = vec![0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(is_valid_dns_message(&valid_dns));

        // 太短的消息
        let short_msg = vec![0x12, 0x34];
        assert!(!is_valid_dns_message(&short_msg));
    }

    #[test]
    fn test_extract_query_id() {
        let dns_msg = vec![0x12, 0x34, 0x01, 0x00];
        assert_eq!(extract_query_id(&dns_msg), Some(0x1234));
        
        let short_msg = vec![0x12];
        assert_eq!(extract_query_id(&short_msg), None);
    }
}
