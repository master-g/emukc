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
		let proxy = reqwest::Proxy::http(proxy)?;
		builder.proxy(proxy)
	} else {
		builder.no_proxy()
	};

	builder.build()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_new_reqwest_client() {
		let client = new_reqwest_client(None, None).unwrap();
		client.get("http://w00g.kancolle-server.com/kcs2/world.html").send().await.unwrap();
	}
}
