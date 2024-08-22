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
		.pool_max_idle_per_host(0)
		.user_agent(ua.unwrap_or(DEFAULT_UA));

	let builder = if let Some(proxy) = proxy {
		let proxy = reqwest::Proxy::all(proxy)?;
		builder.proxy(proxy)
	} else {
		builder
	};

	builder.build()
}
