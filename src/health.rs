//! Vérification de joignabilité de Home Assistant (connexion TCP, sans pile TLS).

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use url::Url;

const TIMEOUT: Duration = Duration::from_secs(3);

/// Vrai si l'hôte:port de `url` accepte une connexion TCP.
pub fn is_reachable(url: &Url) -> bool {
    let (Some(host), Some(port)) = (url.host_str(), url.port_or_known_default()) else {
        return false;
    };
    let Ok(addrs) = (host, port).to_socket_addrs() else { return false };
    addrs.into_iter().any(|addr| TcpStream::connect_timeout(&addr, TIMEOUT).is_ok())
}
