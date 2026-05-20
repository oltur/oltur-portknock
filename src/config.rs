//! Configuration model shared across the program.

use std::net::IpAddr;
use std::time::Duration;

/// Protocol used for each individual knock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Proto {
    Tcp,
    Udp,
}

/// Which mode to run, with its fully parsed settings.
#[derive(Debug)]
pub enum Command {
    /// Contact a sequence of ports in order.
    Knock(KnockConfig),
    /// Scan a range of ports and report which ones are open.
    Sniff(SniffConfig),
    /// Watch for an incoming knock sequence (server side).
    Detect(DetectConfig),
}

/// A fully resolved knock run: which host and ports to knock, and how.
#[derive(Debug)]
pub struct KnockConfig {
    pub host: String,
    pub ports: Vec<u16>,
    pub proto: Proto,
    pub delay: Duration,
    pub timeout: Duration,
}

/// A fully resolved sniff run: which host and port range to scan, and how.
#[derive(Debug)]
pub struct SniffConfig {
    pub host: String,
    pub start: u16,
    pub end: u16,
    pub timeout: Duration,
    /// Number of ports probed concurrently.
    pub jobs: u32,
}

/// A fully resolved detect run: the knock sequence to watch for, and how.
#[derive(Debug)]
pub struct DetectConfig {
    /// Ports that, contacted in this order, count as a recognized knock.
    pub sequence: Vec<u16>,
    /// Address the port listeners bind to.
    pub bind: IpAddr,
    /// How long a client has to complete the whole sequence in order.
    pub window: Duration,
}
