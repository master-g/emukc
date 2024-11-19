use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KcSortieResultRank {
	S,
	A,
	B,
	C,
	D,
	E,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KcSortieResult {
	Any,
	Clear,
	Ranked(KcSortieResultRank),
}
