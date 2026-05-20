//! portknock — send a port-knock sequence to a host.
//!
//! A port-knock "opens" a firewall by contacting a fixed series of ports in
//! order. This client just sends those contacts; the server side decides what
//! the sequence unlocks.

use std::env;
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

const DEFAULT_DELAY_MS: u64 = 200;
const DEFAULT_TIMEOUT_MS: u64 = 200;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Proto {
    Tcp,
    Udp,
}

#[derive(Debug)]
struct Config {
    host: String,
    ports: Vec<u16>,
    proto: Proto,
    delay: Duration,
    timeout: Duration,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args) {
        Ok(Some(cfg)) => match knock(&cfg) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Ok(None) => ExitCode::SUCCESS, // --help was printed
        Err(e) => {
            eprintln!("error: {e}\n");
            print_usage();
            ExitCode::FAILURE
        }
    }
}

/// Parse positional arguments (`<host> <port>...`) and options.
fn parse_args(args: &[String]) -> Result<Option<Config>, String> {
    let mut iter = args.iter();
    let mut host: Option<String> = None;
    let mut ports: Vec<u16> = Vec::new();
    let mut proto = Proto::Tcp;
    let mut delay = Duration::from_millis(DEFAULT_DELAY_MS);
    let mut timeout = Duration::from_millis(DEFAULT_TIMEOUT_MS);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                return Ok(None);
            }
            "--proto" => {
                let v = iter.next().ok_or("--proto requires a value")?;
                proto = match v.as_str() {
                    "tcp" => Proto::Tcp,
                    "udp" => Proto::Udp,
                    other => return Err(format!("unknown proto '{other}' (use tcp or udp)")),
                };
            }
            "--delay" => {
                let v = iter.next().ok_or("--delay requires a value")?;
                let ms = v.parse().map_err(|_| format!("invalid delay '{v}'"))?;
                delay = Duration::from_millis(ms);
            }
            "--timeout" => {
                let v = iter.next().ok_or("--timeout requires a value")?;
                let ms = v.parse().map_err(|_| format!("invalid timeout '{v}'"))?;
                timeout = Duration::from_millis(ms);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option '{other}'"));
            }
            other if host.is_none() => host = Some(other.to_string()),
            other => {
                let port = other.parse().map_err(|_| format!("invalid port '{other}'"))?;
                ports.push(port);
            }
        }
    }

    let host = host.ok_or("missing host")?;
    if ports.is_empty() {
        return Err("at least one port is required".to_string());
    }

    Ok(Some(Config { host, ports, proto, delay, timeout }))
}

/// Send the knock sequence, pausing `delay` between each port.
fn knock(cfg: &Config) -> Result<(), String> {
    for (i, &port) in cfg.ports.iter().enumerate() {
        if i > 0 {
            thread::sleep(cfg.delay);
        }

        let addr = (cfg.host.as_str(), port)
            .to_socket_addrs()
            .map_err(|e| format!("cannot resolve {}: {e}", cfg.host))?
            .next()
            .ok_or_else(|| format!("no address found for {}", cfg.host))?;

        match cfg.proto {
            Proto::Tcp => {
                // The connection is expected to be refused or to time out —
                // reaching the port is the whole point of a knock.
                let _ = TcpStream::connect_timeout(&addr, cfg.timeout);
                println!("knock {port}/tcp -> {}", addr.ip());
            }
            Proto::Udp => {
                let bind = if addr.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
                let socket =
                    UdpSocket::bind(bind).map_err(|e| format!("cannot open udp socket: {e}"))?;
                socket
                    .send_to(&[], addr)
                    .map_err(|e| format!("cannot send udp to port {port}: {e}"))?;
                println!("knock {port}/udp -> {}", addr.ip());
            }
        }
    }

    println!("done: {} knocks sent to {}", cfg.ports.len(), cfg.host);
    Ok(())
}

fn print_usage() {
    println!(
        "portknock — send a port-knock sequence to a host

USAGE:
    portknock <host> <port>... [OPTIONS]

OPTIONS:
    --proto <tcp|udp>   Protocol for each knock (default: tcp)
    --delay <ms>        Delay between knocks (default: {DEFAULT_DELAY_MS})
    --timeout <ms>      TCP connect timeout (default: {DEFAULT_TIMEOUT_MS})
    -h, --help          Show this help

EXAMPLE:
    portknock 192.168.1.10 7000 8000 9000 --delay 300"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_host_and_ports() {
        let cfg = parse_args(&args(&["example.com", "7000", "8000", "9000"]))
            .unwrap()
            .unwrap();
        assert_eq!(cfg.host, "example.com");
        assert_eq!(cfg.ports, vec![7000, 8000, 9000]);
        assert_eq!(cfg.proto, Proto::Tcp);
    }

    #[test]
    fn parses_options() {
        let cfg = parse_args(&args(&["host", "1234", "--proto", "udp", "--delay", "500"]))
            .unwrap()
            .unwrap();
        assert_eq!(cfg.proto, Proto::Udp);
        assert_eq!(cfg.delay, Duration::from_millis(500));
    }

    #[test]
    fn rejects_missing_ports() {
        assert!(parse_args(&args(&["example.com"])).is_err());
    }

    #[test]
    fn rejects_bad_port() {
        assert!(parse_args(&args(&["host", "99999"])).is_err());
    }

    #[test]
    fn help_returns_none() {
        assert!(parse_args(&args(&["--help"])).unwrap().is_none());
    }
}
