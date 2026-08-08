//! HTTP 服务器配置。
//!
//! 对应环境变量前缀 `SERVER_`，例如 `SERVER_HOST`、`SERVER_PORT`。

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use crate::ConfigError;

/// HTTP 服务器监听与超时配置。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    #[serde(default = "ServerConfig::default_host")]
    pub host: IpAddr,

    #[serde(default = "ServerConfig::default_port")]
    pub port: u16,

    /// 请求读取超时（秒）。
    #[serde(default = "ServerConfig::default_read_timeout_secs")]
    pub read_timeout_secs: u64,

    /// 优雅关闭等待时长（秒）：收到关闭信号后，给正在处理的请求留出的
    /// 完成时间，超过后强制中断。
    #[serde(default = "ServerConfig::default_shutdown_grace_secs")]
    pub shutdown_grace_secs: u64,
}

impl ServerConfig {
    const fn default_port() -> u16 {
        8080
    }

    const fn default_read_timeout_secs() -> u64 {
        60
    }

    const fn default_shutdown_grace_secs() -> u64 {
        10
    }

    fn default_host() -> IpAddr {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    }

    /// 监听地址，供 `TcpListener::bind` 直接使用。
    #[must_use]
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    /// 从环境变量加载（前缀 `SERVER_`）。
    pub fn load() -> Result<Self, ConfigError> {
        crate::load_prefixed("SERVER_")
    }

    /// 请求读取超时，转换为 [`Duration`] 供框架 API 使用。
    #[must_use]
    pub const fn read_timeout(&self) -> Duration {
        Duration::from_secs(self.read_timeout_secs)
    }

    /// 优雅关闭等待时长，转换为 [`Duration`]。
    #[must_use]
    pub const fn shutdown_grace_period(&self) -> Duration {
        Duration::from_secs(self.shutdown_grace_secs)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: Self::default_host(),
            port: Self::default_port(),
            read_timeout_secs: Self::default_read_timeout_secs(),
            shutdown_grace_secs: Self::default_shutdown_grace_secs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：测试环境变量加载不要用 `std::env::set_var` + `ServerConfig::load()`。
    // 原因有二：
    // 1. 环境变量是进程级全局状态，多个测试并行跑（Rust 测试默认并行）时会互相
    //    污染，出现测试结果依赖执行顺序的间歇性失败。
    // 2. Rust 2024 edition 起 `std::env::set_var` 本身就是 `unsafe fn`
    //    （多线程下修改环境变量不再被认为是安全操作）。
    //
    // 用 `envy::prefixed(..).from_iter(..)` 直接从内存构造键值对，不触碰真实
    // 环境变量，天然线程安全、可并行、无需清理。

    #[test]
    fn defaults_used_when_no_env_vars_present() {
        let cfg: ServerConfig = envy::prefixed("SERVER_").from_iter(Vec::new()).unwrap();
        assert_eq!(cfg.host, "0.0.0.0".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.read_timeout_secs, 60);
        assert_eq!(cfg.shutdown_grace_secs, 10);
    }

    #[test]
    fn all_fields_loaded_from_prefixed_vars() {
        let vars = vec![
            ("SERVER_HOST".to_string(), "127.0.0.1".to_string()),
            ("SERVER_PORT".to_string(), "9090".to_string()),
            ("SERVER_READ_TIMEOUT_SECS".to_string(), "30".to_string()),
            ("SERVER_SHUTDOWN_GRACE_SECS".to_string(), "5".to_string()),
        ];
        let cfg: ServerConfig = envy::prefixed("SERVER_").from_iter(vars).unwrap();
        assert_eq!(cfg.host, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.port, 9090);
        assert_eq!(cfg.read_timeout_secs, 30);
        assert_eq!(cfg.shutdown_grace_secs, 5);
    }

    #[test]
    fn unprefixed_vars_are_ignored() {
        // 确认前缀隔离生效：不带 SERVER_ 前缀的变量不会被误读进来，
        // 这是多个基础设施配置共存时互不干扰的关键保证。
        let vars = vec![("DATABASE_PORT".to_string(), "5432".to_string())];
        let cfg: ServerConfig = envy::prefixed("SERVER_").from_iter(vars).unwrap();
        // DATABASE_PORT 未被识别为 SERVER_ 变量，port 应落回默认值
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn invalid_port_value_fails_to_parse() {
        let vars = vec![("SERVER_PORT".to_string(), "not-a-number".to_string())];
        let result: Result<ServerConfig, _> = envy::prefixed("SERVER_").from_iter(vars);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_host_value_fails_to_parse() {
        let vars = vec![("SERVER_HOST".to_string(), "invalid-ip".to_string())];
        let result: Result<ServerConfig, _> = envy::prefixed("SERVER_").from_iter(vars);
        assert!(result.is_err());
    }

    #[test]
    fn socket_addr_combines_host_and_port() {
        let cfg = ServerConfig {
            host: "127.0.0.1".parse().unwrap(),
            port: 3000,
            ..ServerConfig::default()
        };
        let expected: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        assert_eq!(cfg.socket_addr(), expected);
    }

    #[test]
    fn read_timeout_converts_seconds_to_duration() {
        let cfg = ServerConfig {
            read_timeout_secs: 45,
            ..ServerConfig::default()
        };
        assert_eq!(cfg.read_timeout(), Duration::from_secs(45));
    }

    #[test]
    fn shutdown_grace_period_converts_seconds_to_duration() {
        let cfg = ServerConfig {
            shutdown_grace_secs: 15,
            ..ServerConfig::default()
        };
        assert_eq!(cfg.shutdown_grace_period(), Duration::from_secs(15));
    }
}
