//! portknock — send a port-knock sequence to a host, or scan a port range.
//!
//! A port-knock "opens" a firewall by contacting a fixed series of ports in
//! order. This client just sends those contacts; the server side decides what
//! the sequence unlocks. The `sniff` mode instead scans a range of TCP ports
//! and reports which ones accept a connection, and `detect` watches for an
//! incoming knock sequence.

mod cli;
mod config;
mod detect;
mod knock;
mod sniff;

use std::env;
use std::process::ExitCode;

use config::Command;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let outcome = match cli::parse_args(&args) {
        Ok(Some(Command::Knock(cfg))) => knock::knock(&cfg),
        Ok(Some(Command::Sniff(cfg))) => sniff::sniff(&cfg),
        Ok(Some(Command::Detect(cfg))) => detect::detect(&cfg),
        Ok(None) => return ExitCode::SUCCESS, // --help was printed
        Err(e) => {
            eprintln!("error: {e}\n");
            cli::print_usage();
            return ExitCode::FAILURE;
        }
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
