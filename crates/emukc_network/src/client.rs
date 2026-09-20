//! Create a new reqwest client

const DEFAULT_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.0.0 Safari/537.36";

/// Create a new reqwest client
///
/// # Arguments
///
/// * `proxy` - The proxy to use for the client
/// * `ua` - The user agent to use for the client
///
/// # Returns
///
/// A new reqwest client, or an error if the client could not be created
pub fn new_reqwest_client(
    proxy: Option<&str>,
    ua: Option<&str>,
) -> Result<reqwest::Client, reqwest::Error> {
    let builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        // A full `cache populate` fetches ~94k files. With no idle connections retained, every
        // one of them pays a fresh TCP + TLS handshake — and a fresh CONNECT when a proxy is
        // configured — which is what dominates the wall clock. Sized to the default populate
        // concurrency (`CONCURRENT ?= 16` in the Makefile).
        .pool_max_idle_per_host(16)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .user_agent(ua.unwrap_or(DEFAULT_UA));

    let builder = if let Some(proxy) = proxy {
        let proxy = reqwest::Proxy::all(proxy)?;
        builder.proxy(proxy)
    } else {
        builder.no_proxy()
    };

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds only. This used to GET a live kancolle-server URL, which made
    /// `cargo test` fail whenever the machine was offline or the proxy was down.
    #[test]
    fn test_new_reqwest_client() {
        assert!(new_reqwest_client(None, None).is_ok());
    }

    #[test]
    fn test_new_reqwest_client_accepts_http_proxy() {
        new_reqwest_client(Some("http://127.0.0.1:1086"), None).unwrap();
    }

    #[test]
    fn test_new_reqwest_client_accepts_socks5_proxy() {
        new_reqwest_client(Some("socks5://127.0.0.1:1086"), None).unwrap();
    }
}
