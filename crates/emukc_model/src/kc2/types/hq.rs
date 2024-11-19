use serde::{Deserialize, Serialize};

#[derive(
	Default, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N,
)]
pub enum UserHQRank {
	MarshalAdmiral = 1,
	Admiral = 2,
	ViceAdmiral = 3,
	RearAdmiral = 4,
	Captain = 5,
	Commander = 6,
	JuniorCommander = 7,
	LieutenantCommander = 8,

	/// this is a special rank for the game, should be 8
	ViceLieutenantCommander = 9,
	#[default]
	JuniorLieutenantCommander = 10,
}

impl UserHQRank {
	pub fn get_name(&self) -> &str {
		match self {
			UserHQRank::MarshalAdmiral => "元帥",
			UserHQRank::Admiral => "大将",
			UserHQRank::ViceAdmiral => "中将",
			UserHQRank::RearAdmiral => "少将",
			UserHQRank::Captain => "大佐",
			UserHQRank::Commander => "中佐",
			UserHQRank::JuniorCommander => "新米中佐",
			UserHQRank::LieutenantCommander => "少佐",
			UserHQRank::ViceLieutenantCommander => "中堅少佐",
			UserHQRank::JuniorLieutenantCommander => "新米少佐",
		}
	}
}

impl From<i64> for UserHQRank {
	fn from(value: i64) -> Self {
		match value {
			1 => UserHQRank::MarshalAdmiral,
			2 => UserHQRank::Admiral,
			3 => UserHQRank::ViceAdmiral,
			4 => UserHQRank::RearAdmiral,
			5 => UserHQRank::Captain,
			6 => UserHQRank::Commander,
			7 => UserHQRank::JuniorCommander,
			_ => UserHQRank::LieutenantCommander,
		}
	}
}
