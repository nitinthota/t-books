use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Real connectivity check used before Google. UI banners use the webview
/// `navigator.onLine` flag so this is not called on every paint.
pub fn is_online() -> bool {
    let Ok(mut addrs) = ("oauth2.googleapis.com", 443).to_socket_addrs() else {
        return false;
    };
    let Some(addr) = addrs.next() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(1500)).is_ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn offline_error_text_is_specific() {
        assert!(!crate::OFFLINE_BANNER.is_empty());
    }
}
