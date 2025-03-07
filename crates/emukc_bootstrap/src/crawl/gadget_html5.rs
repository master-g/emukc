use std::sync::LazyLock;

use emukc_cache::prelude::*;

static JS_LIST: LazyLock<&[&str]> = LazyLock::new(|| {
	&["cda", "const", "content", "global", "inspection", "login", "options", "payment"]
});

static JAVASCRIPT_LIST: LazyLock<&[&str]> =
	LazyLock::new(|| &["cookie", "jquery.min", "jss", "rollover"]);

pub(super) async fn crawl_gadget_html5(cache: &Kache) -> Result<(), KacheError> {
	for js in JS_LIST.iter() {
		cache.get(format!("gadget_html5/js/kcs_{}.js", js).as_str(), "").await?;
	}
	for js in JAVASCRIPT_LIST.iter() {
		cache.get(format!("gadget_html5/script/{}.js", js).as_str(), "").await?;
	}

	Ok(())
}
