//! Benchmark for Kache get operations with version cache.

use criterion::{Criterion, criterion_group, criterion_main};
use emukc_cache::Kache;
use std::hint::black_box;
use tempfile::TempDir;

fn setup_cache() -> (Kache, TempDir) {
	let temp_dir = TempDir::new().unwrap();
	let cache = Kache::builder()
		.with_cache_root(temp_dir.path().to_path_buf())
		.with_content_cdn("http://cdn.com".to_string())
		.with_gadgets_cdn("http://gadgets.com".to_string())
		.build()
		.unwrap();

	std::fs::write(temp_dir.path().join("test.png"), b"test data").unwrap();

	(cache, temp_dir)
}

fn bench_local_file_exists_check(c: &mut Criterion) {
	let (cache, _temp) = setup_cache();

	c.bench_function("local_file_exists", |b| {
		b.iter(|| {
			let rt = tokio::runtime::Runtime::new().unwrap();
			rt.block_on(async {
				let opt = emukc_cache::GetOption::new().disable_remote();
				black_box(cache.get_with_opt("test.png", "", &opt).await)
			})
		});
	});
}

criterion_group!(benches, bench_local_file_exists_check);
criterion_main!(benches);
