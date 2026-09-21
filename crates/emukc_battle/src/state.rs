use crate::combined::{CombinedFleetRole, CombinedType};
use crate::outcome::{calculate_mvp, calculate_win_rank, verify_protected_ships_alive};
use crate::targeting::any_alive;
use crate::types::CombinedMembership;
use crate::types::{
    BattleContext, BattleHougeki, BattleKouku, BattleOpeningAttack, BattleOutcome, BattlePacket,
    BattleRaigeki, BattleRuntimeShip, BattleSimulation, BattleType, NightBattlePacket,
    NightBattleSimulation,
};

/// Combined-fleet layout of [`BattleState::friendly`].
///
/// The two decks live in one contiguous vector — deck 1 at `..escort_start`,
/// deck 2 at `escort_start..` — so that a phase in which the enemy fires at the
/// whole friendly force is just the whole slice, and a phase belonging to one
/// deck is a sub-slice. `escort_start` is deck 1's actual ship count, which is
/// **not** the packet's escort offset: the client puts deck 2 at index 6 no
/// matter how few ships deck 1 holds. The phases work entirely in this space;
/// [`finalize_day`](BattleState::finalize_day) translates the finished packet
/// into the client's through [`combined_packet`](crate::combined_packet).
#[derive(Debug, Clone, Copy)]
pub(crate) struct CombinedLayout {
    pub(crate) combined_type: CombinedType,
    pub(crate) escort_start: usize,
}

/// All mutable state for a single battle simulation.
///
/// Created from a [`BattleContext`], mutated by phase functions, then
/// consumed by [`finalize_day`](Self::finalize_day) or
/// [`finalize_night`](Self::finalize_night) to produce the simulation output.
///
/// `friendly` and `enemy` are `pub(crate)` because every phase function needs
/// `&mut` access to them. All other fields are private with setters.
pub(crate) struct BattleState {
    pub(crate) friendly: Vec<BattleRuntimeShip>,
    pub(crate) enemy: Vec<BattleRuntimeShip>,

    battle_type: BattleType,
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: super::types::EngagementType,

    kouku: Option<BattleKouku>,
    opening_attack: Option<BattleOpeningAttack>,
    opening_taisen: Option<BattleHougeki>,
    hougeki1: Option<BattleHougeki>,
    hougeki2: Option<BattleHougeki>,
    hougeki3: Option<BattleHougeki>,
    raigeki: Option<BattleRaigeki>,

    /// `None` for an ordinary single-fleet battle.
    combined: Option<CombinedLayout>,

    stage_flag: [i64; 3],
    hourai_flag: [i64; 4],
    opening_taisen_flag: i64,
    has_bb_class_at_start: bool,
}

impl BattleState {
    /// Build initial state from a battle context.
    pub fn from_context(context: BattleContext) -> Self {
        let is_sortie = context.is_sortie;
        let mut friendly = context
            .friend_ships
            .into_iter()
            .map(|s| BattleRuntimeShip::new(s, true, is_sortie))
            .collect::<Vec<_>>();

        // A combined fleet appends deck 2 to the same vector and tags both
        // decks. With no combined setup this loop does not run, `friendly` is
        // byte-for-byte what it was before, and every ship keeps
        // `combined_role: None` — which is what keeps the single-fleet RNG
        // stream identical.
        let combined = context.combined.map(|setup| {
            let combined_type = setup.combined_type;
            for ship in &mut friendly {
                ship.combined = Some(CombinedMembership {
                    combined_type,
                    role: CombinedFleetRole::Main,
                });
            }
            let escort_start = friendly.len();
            friendly.extend(setup.escort_ships.into_iter().map(|s| {
                BattleRuntimeShip::new(s, true, is_sortie)
                    .in_combined_fleet(combined_type, CombinedFleetRole::Escort)
            }));
            CombinedLayout {
                combined_type,
                escort_start,
            }
        });
        let enemy = context
            .enemy_ships
            .into_iter()
            .map(|s| BattleRuntimeShip::new(s, false, is_sortie))
            .collect::<Vec<_>>();

        Self {
            friendly,
            enemy,
            battle_type: context.battle_type,
            friendly_formation_id: context.friendly_formation_id,
            enemy_formation_id: context.enemy_formation_id,
            engagement: context.engagement,
            kouku: None,
            opening_attack: None,
            opening_taisen: None,
            hougeki1: None,
            hougeki2: None,
            hougeki3: None,
            raigeki: None,
            combined,
            stage_flag: [0, 0, 0],
            hourai_flag: [0, 0, 0, 0],
            opening_taisen_flag: 0,
            has_bb_class_at_start: false,
        }
    }

    /// Build minimal state for night battle finalization.
    /// Avoids the full `BattleContext` → runtime-ship pipeline when the ships
    /// have already been mutated by day battle phases.
    pub fn for_night(
        friendly: Vec<BattleRuntimeShip>,
        enemy: Vec<BattleRuntimeShip>,
        friendly_formation_id: i64,
        enemy_formation_id: i64,
        engagement: super::types::EngagementType,
    ) -> Self {
        Self {
            friendly,
            enemy,
            battle_type: BattleType::Normal,
            friendly_formation_id,
            enemy_formation_id,
            engagement,
            kouku: None,
            opening_attack: None,
            opening_taisen: None,
            hougeki1: None,
            hougeki2: None,
            hougeki3: None,
            raigeki: None,
            combined: None,
            stage_flag: [0, 0, 0],
            hourai_flag: [0, 0, 0, 0],
            opening_taisen_flag: 0,
            has_bb_class_at_start: false,
        }
    }

    // -- Read accessors (for phase dispatch) --

    pub(crate) fn battle_type(&self) -> BattleType {
        self.battle_type
    }

    pub(crate) fn friendly_formation_id(&self) -> i64 {
        self.friendly_formation_id
    }

    pub(crate) fn enemy_formation_id(&self) -> i64 {
        self.enemy_formation_id
    }

    pub(crate) fn engagement(&self) -> super::types::EngagementType {
        self.engagement
    }

    /// The combined-fleet layout, or `None` in an ordinary single-fleet battle.
    pub(crate) fn combined(&self) -> Option<CombinedLayout> {
        self.combined
    }

    // -- Setters (for phase functions to write outputs) --

    pub(crate) fn set_kouku(&mut self, kouku: BattleKouku) {
        self.kouku = Some(kouku);
    }

    pub(crate) fn set_opening_attack(&mut self, attack: Option<BattleOpeningAttack>) {
        self.opening_attack = attack;
    }

    pub(crate) fn set_opening_taisen(&mut self, taisen: Option<BattleHougeki>) {
        self.opening_taisen = taisen;
    }

    pub(crate) fn set_opening_taisen_flag(&mut self, flag: bool) {
        self.opening_taisen_flag = i64::from(flag);
    }

    pub(crate) fn set_hougeki1(&mut self, hougeki: Option<BattleHougeki>) {
        self.hougeki1 = hougeki;
    }

    pub(crate) fn set_hougeki2(&mut self, hougeki: Option<BattleHougeki>) {
        self.hougeki2 = hougeki;
    }

    /// Only a combined battle fills the third shelling round: `battle` puts
    /// deck 1's second round here, `battle_water` puts deck 2's single round
    /// here. A single-fleet battle leaves it `None`.
    pub(crate) fn set_hougeki3(&mut self, hougeki: Option<BattleHougeki>) {
        self.hougeki3 = hougeki;
    }

    pub(crate) fn set_raigeki(&mut self, raigeki: Option<BattleRaigeki>) {
        self.raigeki = raigeki;
    }

    pub(crate) fn set_stage_flag(&mut self, flags: [i64; 3]) {
        self.stage_flag = flags;
    }

    pub(crate) fn set_hourai_flag(&mut self, index: usize, value: i64) {
        debug_assert!(index < 4, "hourai_flag index out of bounds: {index}");
        self.hourai_flag[index] = value;
    }

    pub(crate) fn set_has_bb_class_at_start(&mut self, value: bool) {
        self.has_bb_class_at_start = value;
    }

    pub(crate) fn has_bb_class_at_start(&self) -> bool {
        self.has_bb_class_at_start
    }

    pub(crate) fn kouku(&self) -> Option<&BattleKouku> {
        self.kouku.as_ref()
    }

    // -- Finalizers --

    /// Consume state, verify invariants, produce the day battle simulation result.
    pub fn finalize_day(self) -> BattleSimulation {
        verify_protected_ships_alive(&self.friendly);

        let can_midnight = matches!(self.battle_type, BattleType::Normal | BattleType::AirBattle)
            && any_alive(&self.friendly)
            && any_alive(&self.enemy);

        let mut packet = BattlePacket {
            formation: [
                self.friendly_formation_id,
                self.enemy_formation_id,
                self.engagement.api_id(),
            ],
            friendly_nowhps: self.friendly.iter().map(|ship| ship.hp().max(0)).collect(),
            enemy_nowhps: self.enemy.iter().map(|ship| ship.hp().max(0)).collect(),
            smoke_type: 0,
            balloon_cell: 0,
            atoll_cell: 0,
            midnight_flag: i64::from(can_midnight),
            search: [1, 1],
            stage_flag: self.stage_flag,
            kouku: self.kouku,
            opening_taisen_flag: self.opening_taisen_flag,
            opening_taisen: self.opening_taisen,
            opening_flag: i64::from(self.opening_attack.is_some()),
            opening_attack: self.opening_attack,
            hourai_flag: self.hourai_flag,
            hougeki1: self.hougeki1,
            hougeki2: self.hougeki2,
            hougeki3: self.hougeki3,
            raigeki: self.raigeki,
        };

        // The phases wrote every friendly index in this module's contiguous
        // space; the client reads 第2艦隊 from index 6. Translating here is the
        // one chokepoint every caller passes through.
        if let Some(layout) = self.combined {
            crate::combined_packet::remap_day_packet(
                &mut packet,
                layout.escort_start,
                self.enemy.len(),
            );
        }

        let outcome = BattleOutcome {
            win_rank: calculate_win_rank(&self.friendly, &self.enemy),
            mvp: calculate_mvp(&self.friendly),
            can_midnight,
        };

        BattleSimulation {
            friendly: self.friendly,
            enemy: self.enemy,
            packet,
            outcome,
        }
    }

    /// Consume state, verify invariants, produce the night battle simulation result.
    pub fn finalize_night(
        self,
        friendly_nowhps: Vec<i64>,
        friendly_maxhps: Vec<i64>,
        enemy_nowhps: Vec<i64>,
        enemy_maxhps: Vec<i64>,
        hougeki: Option<crate::types::BattleNightHougeki>,
    ) -> NightBattleSimulation {
        verify_protected_ships_alive(&self.friendly);

        let outcome = BattleOutcome {
            win_rank: calculate_win_rank(&self.friendly, &self.enemy),
            mvp: calculate_mvp(&self.friendly),
            can_midnight: false,
        };

        let mut packet = NightBattlePacket {
            formation: [
                self.friendly_formation_id,
                self.enemy_formation_id,
                self.engagement.api_id(),
            ],
            friendly_nowhps,
            friendly_maxhps,
            enemy_nowhps,
            enemy_maxhps,
            touch_plane: [-1, -1],
            flare_pos: [-1, -1],
            hougeki,
        };

        // A combined night battle is fought by 第2艦隊 alone, so the friendly
        // vector holds nothing but the escort deck — and the client still
        // expects it at indices 6..=11. The ships carry their own deck tag, so
        // no caller has to declare it.
        if self.friendly.first().is_some_and(BattleRuntimeShip::is_escort_deck) {
            crate::combined_packet::remap_night_packet(&mut packet);
        }

        NightBattleSimulation {
            friendly: self.friendly,
            enemy: self.enemy,
            packet,
            outcome,
        }
    }
}
