/// IP address utilities for validation and CIDR matching.
pub struct IpUtils;

impl IpUtils {
    /// Parse an IPv4 address to a 32-bit integer.
    /// Handles IPv6-mapped IPv4 (e.g., `"::ffff:127.0.0.1"`).
    pub fn ip_to_int(ip: &str) -> Option<u32> {
        if ip.is_empty() {
            return None;
        }

        let ipv4 = if ip.contains("::ffff:") {
            ip.split("::ffff:").nth(1).unwrap_or(ip)
        } else {
            ip
        };

        let parts: Vec<&str> = ipv4.split('.').collect();
        if parts.len() != 4 {
            return None;
        }

        let mut result: u32 = 0;
        for part in parts {
            let octet: u32 = part.parse().ok()?;
            if octet > 255 {
                return None;
            }
            result = (result << 8) + octet;
        }

        Some(result)
    }

    /// Check if a client IP is within an allowed subnet or matches exactly.
    pub fn is_ip_allowed(client_ip: &str, allowed_ips: &[String]) -> bool {
        if allowed_ips.is_empty() {
            return true;
        }

        let normalized_client_ip = if client_ip == "::1" {
            "127.0.0.1"
        } else {
            client_ip
        };

        for allowed in allowed_ips {
            if allowed == "0.0.0.0/0" || allowed == "*" {
                return true;
            }
            if allowed == normalized_client_ip {
                return true;
            }

            // Check CIDR block
            if allowed.contains('/') {
                let parts: Vec<&str> = allowed.split('/').collect();
                if parts.len() != 2 {
                    continue;
                }

                let subnet = parts[0];
                let mask: u32 = match parts[1].parse() {
                    Ok(m) if m <= 32 => m,
                    _ => continue,
                };

                let client_int = match Self::ip_to_int(normalized_client_ip) {
                    Some(i) => i,
                    None => continue,
                };

                let subnet_int = match Self::ip_to_int(subnet) {
                    Some(i) => i,
                    None => continue,
                };

                let network_mask: u32 = if mask == 0 {
                    0
                } else {
                    0xFFFF_FFFFu32 << (32 - mask)
                };

                if (client_int & network_mask) == (subnet_int & network_mask) {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_to_int() {
        assert_eq!(IpUtils::ip_to_int("127.0.0.1"), Some(2130706433));
        assert_eq!(IpUtils::ip_to_int("::ffff:127.0.0.1"), Some(2130706433));
        assert_eq!(IpUtils::ip_to_int(""), None);
        assert_eq!(IpUtils::ip_to_int("invalid"), None);
    }

    #[test]
    fn test_is_ip_allowed() {
        assert!(IpUtils::is_ip_allowed("127.0.0.1", &[]));
        assert!(IpUtils::is_ip_allowed(
            "127.0.0.1",
            &["0.0.0.0/0".to_string()]
        ));
        assert!(IpUtils::is_ip_allowed(
            "192.168.1.5",
            &["192.168.1.0/24".to_string()]
        ));
        assert!(!IpUtils::is_ip_allowed(
            "10.0.0.1",
            &["192.168.1.0/24".to_string()]
        ));
    }
}
