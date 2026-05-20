//! Detecting an incoming knock sequence (server side).
//!
//! This is the counterpart to `knock`: where `knock` *sends* a sequence, this
//! mode *watches* for one. It binds a `TcpListener` on every port in the
//! sequence and tracks each client's progress separately, by source IP, so
//! knocks from different hosts never combine into a false match.
//!
//! Because it works by accepting connections, it only sees knocks aimed at
//! the sequence ports — it cannot observe knocks against ports nothing is
//! listening on. Catching those would need packet capture (a raw socket or
//! libpcap), which this std-only crate deliberately avoids.

use std::collections::HashMap;
use std::io::{self, IsTerminal};
use std::net::{IpAddr, TcpListener};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::DetectConfig;

/// One observed knock: a connection from `src` that landed on `port`.
struct Knock {
    src: IpAddr,
    port: u16,
}

/// A message to the coordinator from one of the worker threads.
enum Event {
    /// An observed knock, forwarded by a port's accept thread.
    Knock(Knock),
    /// A request to stop, sent when the user presses Enter (or closes stdin).
    Shutdown,
}

/// How far one client has progressed through the knock sequence.
struct Progress {
    /// Number of leading sequence ports hit, in order.
    matched: usize,
    /// When this attempt's first port was hit — used to enforce the window.
    started: Instant,
}

/// What a single observed knock did to a client's progress.
#[derive(Debug, PartialEq)]
enum Step {
    /// Hit the expected next port; `matched` of `total` are now done.
    Advanced { matched: usize, total: usize },
    /// Wrong port, but it was the sequence's first — the attempt restarted.
    Restarted { total: usize },
    /// Final port hit in order — the whole sequence is recognized.
    Recognized,
    /// Wrong port; the client had progress, now discarded.
    Reset,
    /// Wrong port, and the client had no progress to lose.
    Ignored,
}

/// Listen for the configured knock sequence and report recognized clients.
///
/// When stdin is a terminal, pressing Enter (or closing stdin) stops detect
/// cleanly, returning `Ok(())`; otherwise it runs until the process is
/// signalled. Returns `Err` only when a port cannot be bound.
pub fn detect(cfg: &DetectConfig) -> Result<(), String> {
    // The sequence may name a port more than once; listen on each port once.
    let mut listen_ports: Vec<u16> = cfg.sequence.clone();
    listen_ports.sort_unstable();
    listen_ports.dedup();

    // One accept thread per port feeds knocks to the coordinator below. Bind
    // every port up front so a failure is reported before we claim to listen.
    let (tx, rx) = mpsc::channel::<Event>();
    for &port in &listen_ports {
        let listener = TcpListener::bind((cfg.bind, port))
            .map_err(|e| format!("cannot listen on {}:{port}: {e}", cfg.bind))?;
        let tx = tx.clone();
        thread::spawn(move || accept_loop(&listener, port, &tx));
    }

    // When stdin is a terminal, a watcher thread turns an Enter keypress (or a
    // closed stdin) into a Shutdown event. With no terminal there is nobody to
    // press Enter, so detect runs until the process is signalled instead.
    let interactive = io::stdin().is_terminal();
    if interactive {
        let tx = tx.clone();
        thread::spawn(move || shutdown_on_input(&tx));
    }
    drop(tx); // Only the worker threads should keep the channel open.

    println!(
        "detect: listening on {} port(s) {:?} for sequence {:?}",
        cfg.bind, listen_ports, cfg.sequence
    );
    let stop_hint = if interactive {
        "press Enter to stop"
    } else {
        "Ctrl+C to stop"
    };
    println!(
        "detect: sequence must complete within {} ms — waiting for knocks ({stop_hint})",
        cfg.window.as_millis()
    );

    // The coordinator owns all client state, so no locking is needed.
    let mut progress: HashMap<IpAddr, Progress> = HashMap::new();
    for event in rx {
        let knock = match event {
            Event::Knock(knock) => knock,
            Event::Shutdown => {
                println!("detect: stopping — no longer listening for knocks");
                return Ok(());
            }
        };
        let step = apply_knock(
            &cfg.sequence,
            cfg.window,
            &mut progress,
            &knock,
            Instant::now(),
        );
        report(&knock, step);
    }

    Err("all port listeners stopped".to_string())
}

/// Accept connections on `port` forever, forwarding each peer's IP as a knock.
fn accept_loop(listener: &TcpListener, port: u16, tx: &Sender<Event>) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let Ok(peer) = stream.peer_addr() else {
            continue;
        };
        // The knock carries no payload — close the connection right away.
        drop(stream);
        let knock = Knock {
            src: peer.ip(),
            port,
        };
        if tx.send(Event::Knock(knock)).is_err() {
            break; // The coordinator is gone; nothing left to do.
        }
    }
}

/// Wait for the user to press Enter (or close stdin), then ask the coordinator
/// to stop. An I/O error on stdin is treated as a stop request too.
fn shutdown_on_input(tx: &Sender<Event>) {
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    let _ = tx.send(Event::Shutdown);
}

/// Apply one observed knock to the per-client progress map.
///
/// Split out from [`detect`] so the matching rules can be unit-tested without
/// any sockets: `now` is passed in rather than read from the clock.
fn apply_knock(
    sequence: &[u16],
    window: Duration,
    progress: &mut HashMap<IpAddr, Progress>,
    knock: &Knock,
    now: Instant,
) -> Step {
    // Drop a half-finished attempt whose window has already elapsed.
    let stale = progress
        .get(&knock.src)
        .is_some_and(|p| now.duration_since(p.started) > window);
    if stale {
        progress.remove(&knock.src);
    }

    let current = progress.get(&knock.src);
    let matched = current.map_or(0, |p| p.matched);

    if knock.port == sequence[matched] {
        // The expected next port — advance, keeping the attempt's start time
        // so the window bounds the whole sequence, not just the last hop.
        let started = current.map_or(now, |p| p.started);
        let matched = matched + 1;
        if matched == sequence.len() {
            progress.remove(&knock.src);
            Step::Recognized
        } else {
            progress.insert(knock.src, Progress { matched, started });
            Step::Advanced {
                matched,
                total: sequence.len(),
            }
        }
    } else if knock.port == sequence[0] {
        // Not the expected port, but the sequence's first — start over at 1.
        progress.insert(
            knock.src,
            Progress {
                matched: 1,
                started: now,
            },
        );
        Step::Restarted {
            total: sequence.len(),
        }
    } else if progress.remove(&knock.src).is_some() {
        Step::Reset
    } else {
        Step::Ignored
    }
}

/// Print a one-line summary of what a knock did.
fn report(knock: &Knock, step: Step) {
    let (src, port) = (knock.src, knock.port);
    match step {
        Step::Advanced { matched, total } => {
            println!("detect: {src} knocked port {port} [{matched}/{total}]");
        }
        Step::Restarted { total } => {
            println!("detect: {src} knocked port {port} [1/{total}] (sequence restarted)");
        }
        Step::Recognized => {
            println!("detect: {src} knocked port {port} — knock sequence recognized");
            // TODO: run a configurable action for `src` here (e.g. open a
            // firewall port) instead of only logging.
        }
        Step::Reset => {
            println!("detect: {src} knocked port {port} — out of sequence, progress reset");
        }
        Step::Ignored => {
            println!("detect: {src} knocked port {port} — out of sequence");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    /// A distinct client IP per test, keyed by the last octet.
    fn client(n: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, n))
    }

    fn knock(src: IpAddr, port: u16) -> Knock {
        Knock { src, port }
    }

    #[test]
    fn full_sequence_in_order_is_recognized() {
        let seq = [7000, 8000, 9000];
        let window = Duration::from_secs(10);
        let mut progress = HashMap::new();
        let now = Instant::now();
        let c = client(1);

        assert_eq!(
            apply_knock(&seq, window, &mut progress, &knock(c, 7000), now),
            Step::Advanced {
                matched: 1,
                total: 3
            }
        );
        assert_eq!(
            apply_knock(&seq, window, &mut progress, &knock(c, 8000), now),
            Step::Advanced {
                matched: 2,
                total: 3
            }
        );
        assert_eq!(
            apply_knock(&seq, window, &mut progress, &knock(c, 9000), now),
            Step::Recognized
        );
        // Progress is cleared once the sequence completes.
        assert!(progress.is_empty());
    }

    #[test]
    fn clients_are_tracked_independently() {
        let seq = [7000, 8000];
        let window = Duration::from_secs(10);
        let mut progress = HashMap::new();
        let now = Instant::now();
        let (a, b) = (client(1), client(2));

        apply_knock(&seq, window, &mut progress, &knock(a, 7000), now);
        // b's knock on 8000 must not borrow a's progress.
        assert_eq!(
            apply_knock(&seq, window, &mut progress, &knock(b, 8000), now),
            Step::Ignored
        );
        // a finishing its own sequence still works.
        assert_eq!(
            apply_knock(&seq, window, &mut progress, &knock(a, 8000), now),
            Step::Recognized
        );
    }

    #[test]
    fn wrong_port_resets_progress() {
        let seq = [7000, 8000, 9000];
        let window = Duration::from_secs(10);
        let mut progress = HashMap::new();
        let now = Instant::now();
        let c = client(1);

        apply_knock(&seq, window, &mut progress, &knock(c, 7000), now);
        assert_eq!(
            apply_knock(&seq, window, &mut progress, &knock(c, 1234), now),
            Step::Reset
        );
        // After a reset the next knock starts from scratch.
        assert_eq!(
            apply_knock(&seq, window, &mut progress, &knock(c, 8000), now),
            Step::Ignored
        );
    }

    #[test]
    fn first_port_restarts_instead_of_resetting() {
        let seq = [7000, 8000, 9000];
        let window = Duration::from_secs(10);
        let mut progress = HashMap::new();
        let now = Instant::now();
        let c = client(1);

        apply_knock(&seq, window, &mut progress, &knock(c, 7000), now);
        // Hitting 7000 again while 8000 was expected restarts at 1/3.
        assert_eq!(
            apply_knock(&seq, window, &mut progress, &knock(c, 7000), now),
            Step::Restarted { total: 3 }
        );
        // The restart counts as 1/3, so 8000 then 9000 still complete it.
        apply_knock(&seq, window, &mut progress, &knock(c, 8000), now);
        assert_eq!(
            apply_knock(&seq, window, &mut progress, &knock(c, 9000), now),
            Step::Recognized
        );
    }

    #[test]
    fn window_bounds_the_whole_sequence() {
        let seq = [7000, 8000, 9000];
        let window = Duration::from_millis(500);
        let mut progress = HashMap::new();
        let start = Instant::now();
        let c = client(1);

        apply_knock(&seq, window, &mut progress, &knock(c, 7000), start);
        // 8000 lands in time and keeps the original start timestamp.
        apply_knock(
            &seq,
            window,
            &mut progress,
            &knock(c, 8000),
            start + Duration::from_millis(300),
        );
        // 9000 arrives 600ms after the *first* knock — past the window.
        assert_eq!(
            apply_knock(
                &seq,
                window,
                &mut progress,
                &knock(c, 9000),
                start + Duration::from_millis(600),
            ),
            Step::Ignored
        );
    }
}
