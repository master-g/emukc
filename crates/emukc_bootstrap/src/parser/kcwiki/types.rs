use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoolOrString {
	Bool(bool),
	String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoolOrInt {
	Bool(bool),
	Int(i64),
}

impl From<BoolOrInt> for Option<i64> {
	fn from(b: BoolOrInt) -> Self {
		match b {
			BoolOrInt::Bool(_) => None,
			BoolOrInt::Int(i) => Some(i),
		}
	}
}

impl From<BoolOrInt> for i64 {
	fn from(b: BoolOrInt) -> Self {
		match b {
			BoolOrInt::Bool(_) => 0,
			BoolOrInt::Int(i) => i,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrInt {
	String(String),
	Int(i64),
}

impl From<StringOrInt> for i64 {
	fn from(b: StringOrInt) -> Self {
		match b {
			StringOrInt::String(s) => s.parse().unwrap(),
			StringOrInt::Int(i) => i,
		}
	}
}
