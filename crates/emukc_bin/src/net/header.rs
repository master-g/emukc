use emukc_internal::prelude::PKG_VERSION;
use http::{HeaderName, HeaderValue};
use tower_http::set_header::SetResponseHeaderLayer;

pub(super) fn add_version_header() -> SetResponseHeaderLayer<HeaderValue> {
	let val = format!("emukc-{}", *PKG_VERSION);
	SetResponseHeaderLayer::if_not_present(
		HeaderName::from_static("emukc_version").to_owned(),
		HeaderValue::try_from(val).unwrap(),
	)
}

static CONTENT_TYPE: HeaderName = HeaderName::from_static("content-type");

pub(super) fn add_content_type_json_header() -> SetResponseHeaderLayer<HeaderValue> {
	SetResponseHeaderLayer::if_not_present(
		CONTENT_TYPE.to_owned(),
		HeaderValue::try_from("application/json; charset=\"utf-8\"").unwrap(),
	)
}
