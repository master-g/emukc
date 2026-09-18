//! Shared imports for route handlers.

pub(in crate::net) use emukc_internal::prelude::*;

pub(in crate::net) use super::{
    AppState,
    auth::{GameSession, Pid},
    resp::{KcApiResponse, KcApiResult},
};
