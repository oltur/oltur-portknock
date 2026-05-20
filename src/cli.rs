//! Command-line argument parsing and help text.

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use crate::config::{Command, DetectConfig, KnockConfig, Proto, SniffConfig};

pub const DEFAULT_DELAY_MS: u64 = 200;
pub const DEFAULT_TIMEOUT_MS: u64 = 200;
pub const DEFAULT_JOBS: u32 = 100;
pub const DEFAULT_WINDOW_MS: u64 = 10_000;

/// Parse the full argument list: a `knock`/`sniff`/`detect` subcommand and its options.
///
/// Returns `Ok(None)` when `--help` was requested and printed.
pub fn parse_args(args: &[String]) -> Result<Option<Command>, String> {
    let mut iter = args.iter();
    let sub = iter
        .next()
        .ok_or("missing subcommand (knock, sniff, or detect); try --help")?;

    match sub.as_str() {
        "-h" | "--help" => {
            print_usage();
            Ok(None)
        }
        "knock" => Ok(parse_knock(iter.as_slice())?.map(Command::Knock)),
        "sniff" => Ok(parse_sniff(iter.as_slice())?.map(Command::Sniff)),
        "detect" => Ok(parse_detect(iter.as_slice())?.map(Command::Detect)),
        other => Err(format!(
            "unknown subcommand '{other}' (use knock, sniff, or detect)"
        )),
    }
}

/// Parse `knock <host> <port>... [OPTIONS]`.
fn parse_knock(args: &[String]) -> Result<Option<KnockConfig>, String> {
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
                let port = other
                    .parse()
                    .map_err(|_| format!("invalid port '{other}'"))?;
                ports.push(port);
            }
        }
    }

    let host = host.ok_or("missing host")?;
    if ports.is_empty() {
        return Err("at least one port is required".to_string());
    }
    Ok(Some(KnockConfig {
        host,
        ports,
        proto,
        delay,
        timeout,
    }))
}

/// Parse `sniff <host> <start>-<end> [OPTIONS]`.
fn parse_sniff(args: &[String]) -> Result<Option<SniffConfig>, String> {
    let mut iter = args.iter();
    let mut host: Option<String> = None;
    let mut range: Option<(u16, u16)> = None;
    let mut timeout = Duration::from_millis(DEFAULT_TIMEOUT_MS);
    let mut jobs = DEFAULT_JOBS;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                return Ok(None);
            }
            "--timeout" => {
                let v = iter.next().ok_or("--timeout requires a value")?;
                let ms = v.parse().map_err(|_| format!("invalid timeout '{v}'"))?;
                timeout = Duration::from_millis(ms);
            }
            "--jobs" => {
                let v = iter.next().ok_or("--jobs requires a value")?;
                jobs = v.parse().map_err(|_| format!("invalid jobs '{v}'"))?;
                if jobs == 0 {
                    return Err("--jobs must be at least 1".to_string());
                }
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option '{other}'"));
            }
            other if host.is_none() => host = Some(other.to_string()),
            other if range.is_none() => range = Some(parse_range(other)?),
            other => return Err(format!("unexpected argument '{other}'")),
        }
    }

    let host = host.ok_or("missing host")?;
    let (start, end) = range.ok_or("missing port range (e.g. 1-1024)")?;
    Ok(Some(SniffConfig {
        host,
        start,
        end,
        timeout,
        jobs,
    }))
}

/// Parse `detect <port>... [OPTIONS]` — the knock sequence to watch for.
fn parse_detect(args: &[String]) -> Result<Option<DetectConfig>, String> {
    let mut iter = args.iter();
    let mut sequence: Vec<u16> = Vec::new();
    let mut bind = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    let mut window = Duration::from_millis(DEFAULT_WINDOW_MS);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                return Ok(None);
            }
            "--bind" => {
                let v = iter.next().ok_or("--bind requires a value")?;
                bind = v
                    .parse()
                    .map_err(|_| format!("invalid bind address '{v}'"))?;
            }
            "--window" => {
                let v = iter.next().ok_or("--window requires a value")?;
                let ms = v.parse().map_err(|_| format!("invalid window '{v}'"))?;
                window = Duration::from_millis(ms);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option '{other}'"));
            }
            other => {
                let port = other
                    .parse()
                    .map_err(|_| format!("invalid port '{other}'"))?;
                sequence.push(port);
            }
        }
    }

    if sequence.is_empty() {
        return Err("at least one port is required".to_string());
    }
    Ok(Some(DetectConfig {
        sequence,
        bind,
        window,
    }))
}

/// Parse a `<start>-<end>` range, or a lone port as a one-port range.
fn parse_range(s: &str) -> Result<(u16, u16), String> {
    match s.split_once('-') {
        Some((a, b)) => {
            let start = a
                .parse()
                .map_err(|_| format!("invalid range start '{a}'"))?;
            let end = b.parse().map_err(|_| format!("invalid range end '{b}'"))?;
            if start > end {
                return Err(format!("range start {start} is greater than end {end}"));
            }
            Ok((start, end))
        }
        None => {
            let port = s.parse().map_err(|_| format!("invalid port '{s}'"))?;
            Ok((port, port))
        }
    }
}

pub fn print_usage() {
    println!(
        "portknock — port-knock client and TCP port scanner

USAGE:
    portknock knock <host> <port>...        Contact ports in sequence
    portknock sniff <host> <start>-<end>    Scan a port range, report open ports
    portknock detect <port>...              Watch for an incoming knock sequence

KNOCK OPTIONS:
    --proto <tcp|udp>   Protocol for each knock (default: tcp)
    --delay <ms>        Delay between knocks (default: {DEFAULT_DELAY_MS})
    --timeout <ms>      TCP connect timeout (default: {DEFAULT_TIMEOUT_MS})

SNIFF OPTIONS:
    --timeout <ms>      TCP connect timeout per port (default: {DEFAULT_TIMEOUT_MS})
    --jobs <n>          Ports probed concurrently (default: {DEFAULT_JOBS})

DETECT OPTIONS:
    --bind <addr>       Address the listeners bind to (default: 0.0.0.0)
    --window <ms>       Time allowed to complete the sequence (default: {DEFAULT_WINDOW_MS})

    -h, --help          Show this help

EXAMPLES:
    portknock knock 192.168.1.10 7000 8000 9000 --delay 300
    portknock sniff 192.168.1.10 1-1024 --jobs 200
    portknock detect 7000 8000 9000 --window 5000"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn knock_cfg(v: &[&str]) -> KnockConfig {
        match parse_args(&args(v)).unwrap().unwrap() {
            Command::Knock(c) => c,
            _ => panic!("expected a knock command"),
        }
    }

    fn sniff_cfg(v: &[&str]) -> SniffConfig {
        match parse_args(&args(v)).unwrap().unwrap() {
            Command::Sniff(c) => c,
            _ => panic!("expected a sniff command"),
        }
    }

    fn detect_cfg(v: &[&str]) -> DetectConfig {
        match parse_args(&args(v)).unwrap().unwrap() {
            Command::Detect(c) => c,
            _ => panic!("expected a detect command"),
        }
    }

    #[test]
    fn parses_host_and_ports() {
        let cfg = knock_cfg(&["knock", "example.com", "7000", "8000", "9000"]);
        assert_eq!(cfg.host, "example.com");
        assert_eq!(cfg.ports, vec![7000, 8000, 9000]);
        assert_eq!(cfg.proto, Proto::Tcp);
    }

    #[test]
    fn parses_options() {
        let cfg = knock_cfg(&["knock", "host", "1234", "--proto", "udp", "--delay", "500"]);
        assert_eq!(cfg.proto, Proto::Udp);
        assert_eq!(cfg.delay, Duration::from_millis(500));
    }

    #[test]
    fn rejects_missing_ports() {
        assert!(parse_args(&args(&["knock", "example.com"])).is_err());
    }

    #[test]
    fn rejects_bad_port() {
        assert!(parse_args(&args(&["knock", "host", "99999"])).is_err());
    }

    #[test]
    fn help_returns_none() {
        assert!(parse_args(&args(&["--help"])).unwrap().is_none());
    }

    #[test]
    fn requires_a_subcommand() {
        assert!(parse_args(&args(&["example.com", "7000"])).is_err());
        assert!(parse_args(&args(&[])).is_err());
    }

    #[test]
    fn parses_sniff_range() {
        let cfg = sniff_cfg(&["sniff", "example.com", "1-1024"]);
        assert_eq!(cfg.host, "example.com");
        assert_eq!((cfg.start, cfg.end), (1, 1024));
        assert_eq!(cfg.jobs, DEFAULT_JOBS);
    }

    #[test]
    fn parses_sniff_single_port() {
        let cfg = sniff_cfg(&["sniff", "host", "22"]);
        assert_eq!((cfg.start, cfg.end), (22, 22));
    }

    #[test]
    fn parses_sniff_options() {
        let cfg = sniff_cfg(&["sniff", "host", "1-10", "--jobs", "50", "--timeout", "300"]);
        assert_eq!(cfg.jobs, 50);
        assert_eq!(cfg.timeout, Duration::from_millis(300));
    }

    #[test]
    fn rejects_reversed_range() {
        assert!(parse_args(&args(&["sniff", "host", "1000-1"])).is_err());
    }

    #[test]
    fn rejects_zero_jobs() {
        assert!(parse_args(&args(&["sniff", "host", "1-10", "--jobs", "0"])).is_err());
    }

    #[test]
    fn rejects_missing_range() {
        assert!(parse_args(&args(&["sniff", "host"])).is_err());
    }

    #[test]
    fn parses_detect_sequence() {
        let cfg = detect_cfg(&["detect", "7000", "8000", "9000"]);
        assert_eq!(cfg.sequence, vec![7000, 8000, 9000]);
        assert_eq!(cfg.bind, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    }

    #[test]
    fn parses_detect_options() {
        let cfg = detect_cfg(&[
            "detect",
            "7000",
            "8000",
            "--bind",
            "127.0.0.1",
            "--window",
            "5000",
        ]);
        assert_eq!(cfg.sequence, vec![7000, 8000]);
        assert_eq!(cfg.bind, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.window, Duration::from_millis(5000));
    }

    #[test]
    fn rejects_detect_without_ports() {
        assert!(parse_args(&args(&["detect"])).is_err());
    }

    #[test]
    fn rejects_bad_bind_address() {
        assert!(parse_args(&args(&["detect", "7000", "--bind", "not-an-ip"])).is_err());
    }
}
