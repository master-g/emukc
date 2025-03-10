use std::sync::LazyLock;

use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use crate::{make_list::CacheList, prelude::CacheListMakingError};

static JS_LIST: LazyLock<&[&str]> = LazyLock::new(|| {
	&["cda", "const", "content", "global", "inspection", "login", "options", "payment"]
});

static JAVASCRIPT_LIST: LazyLock<&[&str]> =
	LazyLock::new(|| &["cookie", "jquery.min", "jss", "rollover"]);

pub(super) async fn make(
	_mst: &ApiManifest,
	_kache: &Kache,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	for js in JS_LIST.iter() {
		list.add_unversioned(format!("gadget_html5/js/kcs_{}.js", js));
	}
	for js in JAVASCRIPT_LIST.iter() {
		list.add_unversioned(format!("gadget_html5/script/{}.js", js));
	}

	Ok(())
}
