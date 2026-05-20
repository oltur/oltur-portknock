# portknock

A small command-line **port-knock client**, **TCP port scanner**, and
**knock detector** — written in Rust, with no external crate dependencies.

[Port knocking](https://en.wikipedia.org/wiki/Port_knocking) hides a service
behind a secret sequence of ports: a firewall stays closed until a client
contacts a fixed series of ports in the right order, then opens. `portknock`
covers both sides of that handshake and throws in a port scanner.

## Features

- **`knock`** — send a knock sequence to a host (TCP or UDP).
- **`sniff`** — scan a range of TCP ports concurrently and report the open ones.
- **`detect`** — the server side: listen for an incoming knock sequence and
  report each client that completes it, with a clean press-Enter-to-stop exit.
- **`#[oltur_trace]`** — a bundled procedural attribute macro that logs when a
  function starts and finishes, with timestamps and elapsed time.
- **Zero external dependencies** — the binary uses only the Rust standard
  library; the macro uses only the built-in `proc_macro` crate.

## Requirements

- Rust **1.85+** (the crate uses edition 2024).
- No other tooling — `cargo build` is enough.

## Build

```bash
cargo build              # debug build  -> target/debug/portknock
cargo build --release    # optimized    -> target/release/portknock
```

Run directly during development with `cargo run -- <args>`:

```bash
cargo run -- knock 127.0.0.1 7000 8000 9000
```

## Usage

```
portknock knock  <host> <port>...        Contact ports in sequence
portknock sniff  <host> <start>-<end>    Scan a port range, report open ports
portknock detect <port>...               Watch for an incoming knock sequence
portknock --help                         Show built-in help
```

The process exits `0` on success and `1` on error (bad arguments, a port that
cannot be bound, an unresolvable host, …).

### `knock` — send a knock sequence

Contacts each port in order, pausing `--delay` between them. For TCP it makes a
connection attempt and **ignores the result on purpose** — simply reaching the
port is the whole point of a knock. For UDP it sends an empty datagram.

| Option | Default | Meaning |
|---|---|---|
| `--proto <tcp\|udp>` | `tcp` | Protocol used for each knock |
| `--delay <ms>` | `200` | Pause between consecutive knocks |
| `--timeout <ms>` | `200` | TCP connect timeout per port |

```bash
$ portknock knock 192.168.1.10 7000 8000 9000 --delay 300
knock 7000/tcp -> 192.168.1.10
knock 8000/tcp -> 192.168.1.10
knock 9000/tcp -> 192.168.1.10
done: 3 knocks sent to 192.168.1.10
```

### `sniff` — scan a TCP port range

Probes every port in the range and reports which ones accept a connection. A
bounded pool of worker threads races for ports off a shared counter, so a wide
range finishes in seconds rather than one timeout per closed port. The range
may be `<start>-<end>` or a single port.

| Option | Default | Meaning |
|---|---|---|
| `--timeout <ms>` | `200` | TCP connect timeout per port |
| `--jobs <n>` | `100` | Ports probed concurrently (must be ≥ 1) |

```bash
$ portknock sniff 192.168.1.10 1-1024 --jobs 200
sniffing 192.168.1.10 (192.168.1.10) ports 1-1024 — 1024 port(s), 200 worker(s)
  22/tcp open
  80/tcp open
  443/tcp open
done: 3 of 1024 port(s) open on 192.168.1.10
```

### `detect` — watch for an incoming knock sequence

The server-side counterpart to `knock`. It binds a TCP listener on every port
in the sequence (duplicate ports are listened on once) and tracks each client's
progress **separately, by source IP** — so knocks from different hosts never
combine into a false match. A client must complete the sequence in order within
`--window`.

| Option | Default | Meaning |
|---|---|---|
| `--bind <addr>` | `0.0.0.0` | Address the listeners bind to |
| `--window <ms>` | `10000` | Time allowed to complete the whole sequence |

```bash
$ portknock detect 7000 8000 9000 --window 5000
detect: listening on 0.0.0.0 port(s) [7000, 8000, 9000] for sequence [7000, 8000, 9000]
detect: sequence must complete within 5000 ms — waiting for knocks (press Enter to stop)
detect: 192.168.1.50 knocked port 7000 [1/3]
detect: 192.168.1.50 knocked port 8000 [2/3]
detect: 192.168.1.50 knocked port 9000 — knock sequence recognized
detect: stopping — no longer listening for knocks
```

**Stopping it:** when run in a terminal, press **Enter** (or Ctrl+D) to stop
cleanly — `detect` returns success and the process exits normally. When stdin
is not a terminal (a service, `nohup`, a redirected pipe), there is no Enter
watcher, so stop it with Ctrl+C or a signal instead.

**Scope:** `detect` only sees knocks aimed at the sequence ports, because it
works by accepting connections. Observing knocks against ports nothing listens
on would require raw packet capture, which this std-only crate deliberately
avoids.

## The `oltur_trace` macro

The workspace includes a second crate, `oltur_trace/`, a procedural-macro crate
that exposes a single attribute macro. Applied to any function, it logs (to
**stderr**) when the function starts and how long it ran:

```rust
use oltur_trace::oltur_trace;

#[oltur_trace]
fn main() -> std::process::ExitCode {
    // ...
}
```

```text
[2026-05-20T14:47:33.798Z] [oltur_trace] main started
[2026-05-20T14:47:33.799Z] [oltur_trace] main finished in 542.25µs
```

Each line is prefixed with an RFC 3339 **UTC** timestamp, computed from `std`
alone (no `chrono`). The "finished" line is emitted from a `Drop` guard, so it
fires on **every** exit path — including an early `return`, a `?`, or a panic
unwinding through the function. In this project `main` carries the attribute.

## Project layout

This repository is a Cargo workspace with two crates:

```
portknock/
├── Cargo.toml          # workspace root + the `portknock` binary package
├── src/
│   ├── main.rs         # entry point; dispatches to a mode, maps to ExitCode
│   ├── cli.rs          # argument parsing and --help text
│   ├── config.rs       # shared model: Command, *Config structs, Proto
│   ├── knock.rs        # knock mode
│   ├── sniff.rs        # sniff mode (worker-pool port scanner)
│   └── detect.rs       # detect mode (knock-sequence listener)
└── oltur_trace/
    └── src/lib.rs      # the #[oltur_trace] attribute macro
```

## Development

```bash
cargo test               # run all tests
cargo test <name>        # run a single test by name
cargo clippy --workspace # lint both crates
cargo fmt                # format
cargo check              # fast type/borrow check, no binary
```

Tests live in `#[cfg(test)]` modules: `cli.rs` covers argument parsing and
`detect.rs` covers the knock-matching rules (kept socket-free so they can be
tested without binding any ports).

## Notes & troubleshooting

- **Privileges:** binding `detect` to a port below 1024 requires root on Unix.
  `knock` and `sniff` make only outbound connections and need no privileges.
- **`Address already in use` on `detect`:** another process already holds that
  port. On macOS, port **7000** is used by the AirPlay Receiver — either pick
  ports outside the well-known range or disable AirPlay Receiver in System
  Settings. Find the holder with `lsof -nP -iTCP:<port> -sTCP:LISTEN`.
- **UDP knocks** send an empty datagram and get no confirmation — a UDP `knock`
  reporting success only means the datagram was handed to the OS.
