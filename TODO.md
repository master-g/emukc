# TODO

should we expose db entities to the outside of the crate?

```rust
pub const KC_DEFAULT_FURNITURES: [i64; 6] = [1, 38, 72, 102, 133, 164];

impl Default for KcApiUserBasic {
	fn default() -> Self {
		Self {
			api_member_id: Default::default(),
			api_nickname: Default::default(),
			api_nickname_id: Default::default(),
			api_active_flag: 1,
			api_starttime: 0,
			api_level: 1,
			api_rank: 10,
			api_experience: 0,
			api_fleetname: None,
			api_comment: Default::default(),
			api_comment_id: Default::default(),
			api_max_chara: 100,
			api_max_slotitem: 497,
			api_max_kagu: 0,
			api_playtime: 0,
			api_tutorial: 0,
			api_furniture: KC_DEFAULT_FURNITURES.to_vec(),
			api_count_deck: 1,
			api_count_kdock: 2,
			api_count_ndock: 2,
			api_fcoin: 0,
			api_st_win: 0,
			api_st_lose: 0,
			api_ms_count: 0,
			api_ms_success: 0,
			api_pt_win: 0,
			api_pt_lose: 0,
			api_pt_challenged: 0,
			api_pt_challenged_win: 0,
			api_firstflag: 0,
			api_tutorial_progress: 0,
			api_pvp: vec![0, 0],
			api_medals: 0,
			api_large_dock: 0,
			api_max_quests: 3,
			api_extra_supply: vec![0, 0],
			api_war_result: 0,
		}
	}
}
```
