//! The ranking (`api_req_ranking/mxltvkpyuklh`): every admiral on this server by the month's
//! 戦果.

use emukc_db::{entity::profile, sea_orm::entity::prelude::*};
use emukc_time::chrono::Utc;

use crate::{err::GameplayError, gameplay::Ctx};

use super::basic::war_result;

/// Admirals on one page of the ranking.
const PAGE_SIZE: usize = 10;

/// One admiral's line in the ranking.
#[derive(Debug, Clone)]
pub struct RankingEntry {
    /// Place, from 1.
    pub rank: i64,
    /// The admiral.
    pub profile: profile::Model,
    /// The month's 戦果.
    pub war_result: i64,
}

/// One page of the ranking.
#[derive(Debug, Clone)]
pub struct RankingPage {
    /// Admirals ranked.
    pub count: i64,
    /// Pages they fill.
    pub page_count: i64,
    /// The page shown, from 1.
    pub page: i64,
    /// Its lines.
    pub entries: Vec<RankingEntry>,
}

impl Ctx {
    /// A page of the ranking: by the month's 戦果, most first, an earlier profile first on a
    /// tie. Page 0 is the one the asking admiral is on.
    pub async fn ranking(&self, profile_id: i64, page: i64) -> Result<RankingPage, GameplayError> {
        let now = Utc::now();
        let mut ranked: Vec<(i64, profile::Model)> = profile::Entity::find()
            .all(&*self.db)
            .await?
            .into_iter()
            .map(|record| (war_result(&record, now), record))
            .collect();
        ranked.sort_by(|(a, x), (b, y)| b.cmp(a).then(x.id.cmp(&y.id)));

        let page = if page > 0 {
            page
        } else {
            let at = ranked.iter().position(|(_, record)| record.id == profile_id).unwrap_or(0);
            (at / PAGE_SIZE) as i64 + 1
        };
        let first = (page as usize - 1) * PAGE_SIZE;
        let entries = ranked
            .iter()
            .enumerate()
            .skip(first)
            .take(PAGE_SIZE)
            .map(|(at, (war_result, record))| RankingEntry {
                rank: at as i64 + 1,
                profile: record.clone(),
                war_result: *war_result,
            })
            .collect();
        Ok(RankingPage {
            count: ranked.len() as i64,
            page_count: ranked.len().div_ceil(PAGE_SIZE) as i64,
            page,
            entries,
        })
    }
}
