//! Client view operations.
//!
//! One operation per client view: it reads everything the view shows, in a
//! fixed order, and returns a domain structure. The HTTP layer only projects
//! that structure onto the wire format (plan 002 KD4, KTD5).

pub use require_info::RequireInfoView;

mod require_info;
