//! 端口 & 健康检查工具

use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::time::Duration;

/// 探测 127.0.0.1:port 是否有进程监听（100ms 超时）
pub fn is_port_in_use(port: u16) -> bool {
    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    TcpStream::connect_timeout(&addr.into(), Duration::from_millis(100)).is_ok()
}

/// 探测 127.0.0.1:port 上运行的是不是真的 dsh。
/// 目前判据：GET / 能在 2s 内返回 200/3xx；后续可以换成 dsh 特有的 /api/health。
pub async fn is_dsh_alive(port: u16) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .no_proxy()
        .build();
    let Ok(client) = client else { return false };
    match client.get(format!("http://127.0.0.1:{port}/")).send().await {
        Ok(resp) => {
            let s = resp.status();
            s.is_success() || s.is_redirection()
        }
        Err(_) => false,
    }
}
