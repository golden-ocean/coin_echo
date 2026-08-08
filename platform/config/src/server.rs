//! HTTP 服务器配置。
//!
//! 对应环境变量前缀 `SERVER_`，例如 `SERVER_HOST`、`SERVER_PORT`。

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use crate::ConfigMeta;

/// 配置语义层面的非法状态。
#[derive(Debug, thiserror::Error)]
pub enum ServerConfigError {
    #[error("port 不能为 0")]
    ZeroPort,

    #[error("read_timeout_secs 不能为 0（容易引发 Slowloris 攻击或连接悬挂）")]
    ZeroReadTimeout,

    #[error("shutdown_grace_secs 过大（当前为 {0} 秒，不建议超过 120 秒）")]
    ShutdownGraceTooLarge(u64),
}

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

impl ConfigMeta for ServerConfig {
    type Error = ServerConfigError;

    /// 环境变量前缀
    fn prefix() -> &'static str {
        "SERVER_"
    }

    /// 启动阶段自我验证，失败即终止启动。
    fn validate(&self) -> Result<(), Self::Error> {
        if self.port == 0 {
            return Err(ServerConfigError::ZeroPort);
        }

        if self.read_timeout_secs == 0 {
            return Err(ServerConfigError::ZeroReadTimeout);
        }

        // 避免优雅关闭等待时间长于容器/K8s 强杀周期 (通常 30s ~ 120s)
        if self.shutdown_grace_secs > 120 {
            return Err(ServerConfigError::ShutdownGraceTooLarge(
                self.shutdown_grace_secs,
            ));
        }

        Ok(())
    }
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

    #[test]
    fn valid_config_passes_validation() {
        let cfg = ServerConfig::default();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn zero_port_rejected() {
        let cfg = ServerConfig {
            port: 0,
            ..ServerConfig::default()
        };
        assert!(matches!(cfg.validate(), Err(ServerConfigError::ZeroPort)));
    }

    #[test]
    fn zero_read_timeout_rejected() {
        let cfg = ServerConfig {
            read_timeout_secs: 0,
            ..ServerConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(ServerConfigError::ZeroReadTimeout)
        ));
    }

    #[test]
    fn excessively_large_shutdown_grace_rejected() {
        let cfg = ServerConfig {
            shutdown_grace_secs: 300,
            ..ServerConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(ServerConfigError::ShutdownGraceTooLarge(300))
        ));
    }

    #[test]
    fn defaults_used_when_no_env_vars_present() {
        let cfg: ServerConfig = envy::prefixed(ServerConfig::prefix())
            .from_iter(Vec::new())
            .unwrap();
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
        let cfg: ServerConfig = envy::prefixed(ServerConfig::prefix())
            .from_iter(vars)
            .unwrap();
        assert_eq!(cfg.host, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.port, 9090);
        assert_eq!(cfg.read_timeout_secs, 30);
        assert_eq!(cfg.shutdown_grace_secs, 5);
        assert!(cfg.validate().is_ok());
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
}
