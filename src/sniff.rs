//! Scanning a range of TCP ports and reporting which ones accept a connection.

use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread;

use crate::config::SniffConfig;

/// Probe every port in the configured range and report the open ones.
///
/// A bounded pool of worker threads pulls ports from a shared counter, so a
/// wide range finishes in seconds instead of one connect-timeout per port.
pub fn sniff(cfg: &SniffConfig) -> Result<(), String> {
    let ip = resolve(&cfg.host)?;

    let total = u32::from(cfg.end) - u32::from(cfg.start) + 1;
    let jobs = cfg.jobs.min(total);

    println!(
        "sniffing {} ({ip}) ports {}-{} — {total} port(s), {jobs} worker(s)",
        cfg.host, cfg.start, cfg.end
    );

    // Workers race to claim ports off this counter; closed ports cost a full
    // connect-timeout, so a shared counter balances the load better than
    // handing each worker a fixed chunk.
    let next = Arc::new(AtomicU32::new(u32::from(cfg.start)));
    let end = u32::from(cfg.end);
    let timeout = cfg.timeout;
    let (tx, rx) = mpsc::channel::<u16>();

    let mut workers = Vec::with_capacity(jobs as usize);
    for _ in 0..jobs {
        let next = Arc::clone(&next);
        let tx = tx.clone();
        workers.push(thread::spawn(move || {
            loop {
                let port = next.fetch_add(1, Ordering::Relaxed);
                if port > end {
                    break;
                }
                let addr = SocketAddr::new(ip, port as u16);
                if TcpStream::connect_timeout(&addr, timeout).is_ok() {
                    // The receiver outlives every worker, so this never fails.
                    let _ = tx.send(port as u16);
                }
            }
        }));
    }
    drop(tx); // Drop our sender so `rx` ends once the workers' clones are gone.

    let mut open: Vec<u16> = rx.iter().collect();
    for worker in workers {
        let _ = worker.join();
    }
    open.sort_unstable();

    for port in &open {
        println!("  {port}/tcp open");
    }
    println!(
        "done: {} of {total} port(s) open on {}",
        open.len(),
        cfg.host
    );
    Ok(())
}

/// Resolve `host` to a single IP address.
fn resolve(host: &str) -> Result<IpAddr, String> {
    // Port 0 is a throwaway — `to_socket_addrs` just needs one to resolve.
    (host, 0)
        .to_socket_addrs()
        .map_err(|e| format!("cannot resolve {host}: {e}"))?
        .next()
        .map(|addr| addr.ip())
        .ok_or_else(|| format!("no address found for {host}"))
}
