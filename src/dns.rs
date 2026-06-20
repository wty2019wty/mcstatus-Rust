//! DNS resolution utilities.
//!
//! Provides SRV record lookup for Minecraft's `_minecraft._tcp.<host>` convention,
//! and A record resolution for hostname-to-IP mapping.

use crate::address::Address;

/// Resolves a Minecraft server address with optional SRV record lookup.
///
/// This mimics the Minecraft client's server address field behavior:
/// if no port is specified, an SRV record lookup is performed for
/// `_minecraft._tcp.<hostname>`. If found, uses the target host/port.
/// Otherwise falls back to `default_port`.
pub async fn minecraft_srv_address_lookup(
    address: &str,
    default_port: u16,
    skip_srv: bool,
) -> Result<Address, String> {
    let addr = Address::parse_address(address, default_port)?;

    // If the caller explicitly asked to skip SRV, return the parsed address as-is
    if skip_srv {
        return Ok(addr);
    }

    // If a specific port was given in the address, don't do SRV lookup
    if address.contains(':') && !address.starts_with('[') {
        return Ok(addr);
    }
    // For IPv6, check if port was specified after the bracket
    if address.starts_with('[') {
        if let Some(bracket_end) = address.find(']') {
            let rest = &address[bracket_end + 1..];
            if rest.starts_with(':') {
                return Ok(addr);
            }
        }
    }

    // Perform SRV record lookup (using hickory-resolver)
    match resolve_mc_srv(&addr.host).await {
        Ok(Some(srv_addr)) => Ok(srv_addr),
        Ok(None) => Ok(addr), // No SRV record, use default
        Err(_) => Ok(addr),   // DNS failure, fall back to default
    }
}

/// Resolves the `_minecraft._tcp.<host>` SRV record.
///
/// Returns `Some(Address)` if an SRV record is found, `None` if not.
pub async fn resolve_mc_srv(host: &str) -> Result<Option<Address>, String> {
    let srv_host = format!("_minecraft._tcp.{host}");
    match async_resolve_srv_record(&srv_host).await? {
        Some((target, port)) => Ok(Some(Address::new(target, port))),
        None => Ok(None),
    }
}

/// Resolves an SRV record, returning the target host and port.
pub async fn async_resolve_srv_record(
    fqdn: &str,
) -> Result<Option<(String, u16)>, String> {
    use hickory_resolver::TokioResolver;
    use hickory_resolver::proto::rr::{RData, RecordType};

    let resolver = TokioResolver::builder_tokio()
        .map_err(|e| format!("Failed to create resolver: {e}"))?
        .build();

    let response = resolver
        .lookup(fqdn, RecordType::SRV)
        .await
        .map_err(|e| format!("SRV lookup failed: {e}"))?;

    for record in response.record_iter() {
        if let RData::SRV(srv) = record.data() {
            let target = srv.target().to_string().trim_end_matches('.').to_string();
            let port = srv.port();
            return Ok(Some((target, port)));
        }
    }

    Ok(None)
}

/// Resolves an A record for the given hostname.
pub async fn async_resolve_a_record(host: &str) -> Result<Vec<std::net::IpAddr>, String> {
    use hickory_resolver::TokioResolver;

    let resolver = TokioResolver::builder_tokio()
        .map_err(|e| format!("Failed to create resolver: {e}"))?
        .build();

    let response = resolver
        .lookup_ip(host)
        .await
        .map_err(|e| format!("A record lookup failed: {e}"))?;

    Ok(response.iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_minecraft_srv_with_explicit_port() {
        let result = minecraft_srv_address_lookup("example.com:25565", 25565, false)
            .await
            .unwrap();
        assert_eq!(result.host, "example.com");
        assert_eq!(result.port, 25565);
    }

    #[tokio::test]
    async fn test_minecraft_srv_without_port() {
        let result = minecraft_srv_address_lookup("localhost", 25565, false)
            .await
            .unwrap();
        assert_eq!(result.host, "localhost");
        assert_eq!(result.port, 25565);
    }

    #[tokio::test]
    async fn test_minecraft_srv_skip() {
        // With skip_srv=true, SRV lookup is skipped entirely
        let result = minecraft_srv_address_lookup("mc.example.com", 25565, true)
            .await
            .unwrap();
        assert_eq!(result.host, "mc.example.com");
        assert_eq!(result.port, 25565);
    }
}
