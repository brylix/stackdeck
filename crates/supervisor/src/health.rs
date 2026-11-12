//! Readiness probes.
//!
//! A process being alive says nothing about whether it is usable — MariaDB
//! takes seconds to open its socket, PHP-FPM to bind. Start order and the
//! "is it up yet" gate both depend on these.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time::{Duration, Instant};

/// One TCP connect attempt.
pub fn tcp_ready(host: &str, port: u16, timeout: Duration) -> bool {
    match (host, port).to_socket_addrs() {
        Ok(mut addrs) => addrs.any(|addr| TcpStream::connect_timeout(&addr, timeout).is_ok()),
        Err(_) => false,
    }
}

/// Poll until the port accepts connections or `deadline` elapses.
pub fn wait_tcp(host: &str, port: u16, deadline: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if tcp_ready(host, port, Duration::from_millis(250)) {
            return true;
        }
        thread::sleep(Duration::from_millis(150));
    }
    false
}

/// Minimal `GET` probe for a known-good path, e.g. Caddy's `/healthz`.
///
/// Deliberately hand-rolled: no TLS, no redirects, no dependency.
pub fn http_ok(url: &str, timeout: Duration) -> bool {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => match p.parse::<u16>() {
            Ok(p) => (h, p),
            Err(_) => return false,
        },
        None => (authority, 80),
    };

    let Ok(mut addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    let Some(addr) = addrs.next() else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, timeout) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 32];
    match stream.read(&mut buf) {
        Ok(n) if n > 12 => buf.starts_with(b"HTTP/1.1 2") || buf.starts_with(b"HTTP/1.0 2"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_port_is_not_ready() {
        assert!(!tcp_ready("127.0.0.1", 1, Duration::from_millis(200)));
    }

    #[test]
    fn deadline_is_respected() {
        let start = Instant::now();
        assert!(!wait_tcp("127.0.0.1", 1, Duration::from_millis(300)));
        assert!(start.elapsed() >= Duration::from_millis(300));
    }
}
