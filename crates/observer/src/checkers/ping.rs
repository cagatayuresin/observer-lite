use std::io::Read;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, Socket, Type};

use super::CheckOutcome;

pub fn extract_host(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return host.to_string();
        }
    }
    url.to_string()
}

pub fn extract_port(url: &str, default: u16) -> u16 {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.port())
        .unwrap_or(default)
}

pub async fn ping_check(url: &str, timeout_seconds: i64, warning_ms: i64) -> CheckOutcome {
    let host = extract_host(url);
    if host.is_empty() || host.starts_with('-') || host.chars().any(char::is_whitespace) {
        return CheckOutcome::down(0, "Invalid ping host");
    }
    let timeout = Duration::from_secs(timeout_seconds.max(1) as u64);
    let host_for_icmp = host.clone();
    let icmp = tokio::task::spawn_blocking(move || icmp_echo(&host_for_icmp, timeout)).await;
    if let Ok(Ok(ms)) = icmp {
        return up_or_warn(ms, warning_ms);
    }
    ping_binary(&host, timeout, warning_ms).await
}

fn up_or_warn(ms: i64, warning_ms: i64) -> CheckOutcome {
    CheckOutcome {
        status: if ms > warning_ms { "warning" } else { "up" }.to_string(),
        response_time_ms: Some(ms),
        ..CheckOutcome::default()
    }
}

/// Unprivileged ICMP echo. Works when `net.ipv4.ping_group_range` includes this user.
fn icmp_echo(host: &str, timeout: Duration) -> Result<i64, String> {
    let addr = format!("{host}:0")
        .to_socket_addrs()
        .map_err(|err| err.to_string())?
        .find(SocketAddr::is_ipv4)
        .ok_or_else(|| "no ipv4 address".to_string())?;
    let mut socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4))
        .map_err(|err| err.to_string())?;
    socket
        .set_read_timeout(Some(timeout))
        .map_err(|err| err.to_string())?;
    socket
        .set_write_timeout(Some(timeout))
        .map_err(|err| err.to_string())?;
    socket
        .connect(&addr.into())
        .map_err(|err| err.to_string())?;

    let ident = std::process::id() as u16;
    let packet = echo_request(ident, 1);
    let started = Instant::now();
    socket.send(&packet).map_err(|err| err.to_string())?;
    let mut buf = [0u8; 256];
    let size = socket.read(&mut buf).map_err(|err| err.to_string())?;
    if size == 0 || buf[0] != 0 {
        return Err("unexpected icmp reply".into());
    }
    Ok(i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX))
}

pub fn echo_request(ident: u16, seq: u16) -> Vec<u8> {
    let mut packet = vec![8, 0, 0, 0];
    packet.extend_from_slice(&ident.to_be_bytes());
    packet.extend_from_slice(&seq.to_be_bytes());
    packet.extend_from_slice(b"observer-lite");
    let sum = checksum(&packet);
    packet[2..4].copy_from_slice(&sum.to_be_bytes());
    packet
}

pub fn checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut index = 0;
    while index + 1 < data.len() {
        sum += u16::from_be_bytes([data[index], data[index + 1]]) as u32;
        index += 2;
    }
    if index < data.len() {
        sum += (data[index] as u32) << 8;
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

async fn ping_binary(host: &str, timeout: Duration, warning_ms: i64) -> CheckOutcome {
    let seconds = timeout.as_secs().max(1).to_string();
    let started = Instant::now();
    let mut command = tokio::process::Command::new("ping");
    command
        .args(["-c", "1", "-W", &seconds, host])
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let result = tokio::time::timeout(timeout + Duration::from_secs(2), command.output()).await;
    let elapsed = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    match result {
        Err(_) => CheckOutcome::down(elapsed, "Ping timed out"),
        Ok(Err(err)) => CheckOutcome::down(elapsed, truncate(&err.to_string())),
        Ok(Ok(output)) if !output.status.success() => {
            CheckOutcome::down(elapsed, format!("Ping failed for {host}"))
        }
        Ok(Ok(output)) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let ms = parse_rtt(&text).unwrap_or(elapsed);
            up_or_warn(ms, warning_ms)
        }
    }
}

pub fn parse_rtt(output: &str) -> Option<i64> {
    let lower = output.to_ascii_lowercase();
    let marker = lower
        .find("time=")
        .map(|i| i + 5)
        .or_else(|| lower.find("time<").map(|i| i + 5))?;
    let rest = &lower[marker..];
    let number: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
        .collect();
    let value: f64 = number.parse().ok()?;
    Some(value as i64)
}

fn truncate(value: &str) -> String {
    value.chars().take(255).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_and_rtt_parsing() {
        assert_eq!(
            extract_host("https://example.com:8443/health"),
            "example.com"
        );
        assert_eq!(extract_port("https://example.com:8443/health", 443), 8443);
        assert_eq!(extract_host("example.com"), "example.com");
        assert_eq!(parse_rtt("64 bytes time=12.9 ms"), Some(12));
        assert_eq!(parse_rtt("time<1 ms"), Some(1));
        let packet = echo_request(1, 1);
        assert_eq!(packet[0], 8);
        assert_eq!(checksum(&packet), 0);
        assert_eq!(checksum(&[1]), checksum(&[1]));
    }

    #[tokio::test]
    async fn invalid_host_and_loopback() {
        let invalid = ping_check(" -bad", 1, 1).await;
        assert_eq!(invalid.status, "down");
        let empty = ping_check("", 1, 1).await;
        assert_eq!(empty.status, "down");
        let loopback = ping_check("127.0.0.1", 2, 10_000).await;
        assert_ne!(loopback.status, "pending");
        let _ = ping_check("127.0.0.1", 2, -1).await;
    }
}
