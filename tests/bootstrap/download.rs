//! An example of downloading bootstrap files

use emukc::prelude::*;

#[tokio::main]
async fn main() {
	download_all().await.unwrap();
}
