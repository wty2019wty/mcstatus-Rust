//! Address parsing and IP resolution.

use std::net::IpAddr;
use std::sync::OnceLock;

/// Represents a resolved server address with host and port.
#[derive(Debug, Clone)]
pub struct Address {
    /// The hostname or IP address.
    pub host: String,
    /// The port number.
    pub port: u16,
    /// Cached resolved IP address.
    cached_ip: OnceLock<IpAddr>,
}

impl Address {
    /// Creates a new address with the given host and port.
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            cached_ip: OnceLock::new(),
        }
    }

    /// Parses an address string in the format `host[:port]`.
    ///
    /// If no port is specified, `default_port` is used.
    /// Returns an error for invalid addresses.
    pub fn parse_address(address: &str, default_port: u16) -> Result<Self, String> {
        // Handle IPv6 addresses: [::1]:25565 or just [::1]
        if address.starts_with('[') {
            if let Some(bracket_end) = address.find(']') {
                let host = &address[1..bracket_end];
                let rest = &address[bracket_end + 1..];
                if rest.is_empty() {
                    return Ok(Self::new(host.to_string(), default_port));
                }
                if rest.starts_with(':') {
                    let port_str = &rest[1..];
                    let port: u16 = port_str
                        .parse()
                        .map_err(|_| format!("Invalid port: '{port_str}'"))?;
                    return Ok(Self::new(host.to_string(), port));
                }
                return Err(format!("Invalid address format: '{address}'"));
            }
        }

        // Standard host:port or host format
        let parts: Vec<&str> = address.rsplitn(2, ':').collect();
        match parts.len() {
            1 => {
                // No port specified
                let host = parts[0].to_string();
                if host.is_empty() {
                    return Err("Address cannot be empty".into());
                }
                Ok(Self::new(host, default_port))
            }
            2 => {
                let host = parts[1].to_string();
                let port: u16 = parts[0]
                    .parse()
                    .map_err(|_| format!("Invalid port: '{}'", parts[0]))?;
                if host.is_empty() {
                    // Case like ":25565" — just port specified
                    return Err("Host cannot be empty".into());
                }
                Ok(Self::new(host, port))
            }
            _ => Err(format!("Invalid address format: '{address}'")),
        }
    }

    /// Resolves the hostname to an IP address.
    ///
    /// IPv6 addresses and already-resolved IPs are returned directly.
    /// Results are cached for subsequent calls.
    pub fn resolve_ip(&self) -> std::io::Result<IpAddr> {
        if let Some(ip) = self.cached_ip.get() {
            return Ok(*ip);
        }

        // Check if it's already an IP address
        if let Ok(ip) = self.host.parse::<IpAddr>() {
            // Store in cache (ignore error if already set by another thread)
            let _ = self.cached_ip.set(ip);
            return Ok(ip);
        }

        // DNS resolution
        let addrs: Vec<std::net::SocketAddr> =
            std::net::ToSocketAddrs::to_socket_addrs(&(self.host.as_str(), self.port))?;

        for addr in addrs {
            let ip = addr.ip();
            let _ = self.cached_ip.set(ip);
            return Ok(ip);
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Could not resolve hostname: {}", self.host),
        ))
    }

    /// Asynchronously resolves the hostname to an IP address.
    pub async fn async_resolve_ip(&self) -> std::io::Result<IpAddr> {
        if let Some(ip) = self.cached_ip.get() {
            return Ok(*ip);
        }

        // Check if it's already an IP address
        if let Ok(ip) = self.host.parse::<IpAddr>() {
            let _ = self.cached_ip.set(ip);
            return Ok(ip);
        }

        // Async DNS resolution
        let host = self.host.clone();
        let result = tokio::task::spawn_blocking(move || {
            (host.as_str(), 0).to_socket_addrs()
        })
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        match result {
            Ok(addrs) => {
                for addr in addrs {
                    let ip = addr.ip();
                    let _ = self.cached_ip.set(ip);
                    return Ok(ip);
                }
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Could not resolve hostname: {}", self.host),
                ))
            }
            Err(e) => Err(e),
        }
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use brackets for IPv6
        if self.host.contains(':') {
            write!(f, "[{}]:{}", self.host, self.port)
        } else {
            write!(f, "{}:{}", self.host, self.port)
        }
    }
}

// Helper trait to enable .to_socket_addrs() on (&str, u16)
use std::net::ToSocketAddrs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address_with_port() {
        let addr = Address::parse_address("example.com:25565", 25565).unwrap();
        assert_eq!(addr.host, "example.com");
        assert_eq!(addr.port, 25565);
    }

    #[test]
    fn test_parse_address_default_port() {
        let addr = Address::parse_address("example.com", 25565).unwrap();
        assert_eq!(addr.host, "example.com");
        assert_eq!(addr.port, 25565);
    }

    #[test]
    fn test_parse_address_custom_default() {
        let addr = Address::parse_address("example.com", 19132).unwrap();
        assert_eq!(addr.port, 19132);
    }

    #[test]
    fn test_parse_address_ipv6() {
        let addr = Address::parse_address("[::1]:25565", 25565).unwrap();
        assert_eq!(addr.host, "::1");
        assert_eq!(addr.port, 25565);
    }

    #[test]
    fn test_parse_address_ipv6_no_port() {
        let addr = Address::parse_address("[::1]", 25565).unwrap();
        assert_eq!(addr.host, "::1");
        assert_eq!(addr.port, 25565);
    }

    #[test]
    fn test_parse_address_ipv4() {
        let addr = Address::parse_address("127.0.0.1:25565", 25565).unwrap();
        assert_eq!(addr.host, "127.0.0.1");
        assert_eq!(addr.port, 25565);
    }

    #[test]
    fn test_parse_address_empty() {
        assert!(Address::parse_address("", 25565).is_err());
    }

    #[test]
    fn test_parse_address_invalid_port() {
        assert!(Address::parse_address("example.com:port", 25565).is_err());
        assert!(Address::parse_address("example.com:99999", 25565).is_err());
    }

    #[test]
    fn test_display() {
        let addr = Address::new("example.com".into(), 25565);
        assert_eq!(addr.to_string(), "example.com:25565");

        let addr_ipv6 = Address::new("::1".into(), 25565);
        assert_eq!(addr_ipv6.to_string(), "[::1]:25565");
    }

    #[test]
    fn test_resolve_ip_localhost() {
        let addr = Address::new("127.0.0.1".into(), 25565);
        let ip = addr.resolve_ip().unwrap();
        assert_eq!(ip, IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
    }
}
