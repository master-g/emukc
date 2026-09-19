//! Client view operations.
//!
//! One operation per client view: it reads everything the view shows, in a
//! fixed order, and returns a domain structure. The HTTP layer only projects
//! that structure onto the wire format (plan 002 KD4, KTD5).

pub use port::PortView;
pub use quest_list::{QuestListItem, QuestListView};
pub use require_info::RequireInfoView;

mod port;
mod quest_list;
mod require_info;
