use axum::{
	async_trait,
	body::Body,
	extract::{FromRef, FromRequest, FromRequestParts, Request},
	middleware::Next,
	response::{IntoResponse, Response},
	Form, RequestPartsExt,
};
use emukc_internal::model::profile::Profile;
use http::{header, request::Parts, StatusCode};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};

use super::AppState;

#[derive(Clone)]
pub(super) struct AuthUserProfile(pub Profile);
