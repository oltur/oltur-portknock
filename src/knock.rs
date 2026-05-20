//! Sending the knock sequence over the network.

use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::thread;

use crate::config::{KnockConfig, Proto};

/// Send the knock sequence, pausing `delay` between each port.
pub fn knock(cfg: &KnockConfig) -> Result<(), String> {
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
                let bind = if addr.is_ipv6() {
                    "[::]:0"
                } else {
                    "0.0.0.0:0"
                };
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
