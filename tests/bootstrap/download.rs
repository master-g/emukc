//! An example of downloading bootstrap files

use emukc::prelude::*;

#[tokio::main]
async fn main() {
	let mut dir = std::path::PathBuf::from(".data");
	dir.push("temp");
	download_all(dir, true, Some("http://127.0.0.1:1080")).await.unwrap();
}
