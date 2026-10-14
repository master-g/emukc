//! Quest progress record
use crate::entity::create_table;
use emukc_model::prelude::Kc3rdQuestPeriod;
use emukc_time::KcTime;
use sea_orm::entity::prelude::*;

pub mod oneshot;
pub mod periodic;
pub mod progress;

#[expect(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Period {
    /// Oneshot
    #[sea_orm(num_value = 1)]
    Oneshot,

    /// Daily
    #[sea_orm(num_value = 2)]
    Daily,

    /// Weekly
    #[sea_orm(num_value = 3)]
    Weekly,

    /// Daily3rd7th0th
    #[sea_orm(num_value = 4)]
    Daily3rd7th0th,

    /// Daily2nd8th
    #[sea_orm(num_value = 5)]
    Daily2nd8th,

    /// Monthly
    #[sea_orm(num_value = 6)]
    Monthly,

    /// Quarterly
    #[sea_orm(num_value = 7)]
    Quarterly,

    /// Annually
    #[sea_orm(num_value = 8)]
    Annually,
}

impl From<Period> for Kc3rdQuestPeriod {
    fn from(value: Period) -> Self {
        match value {
            Period::Oneshot => Kc3rdQuestPeriod::Oneshot,
            Period::Daily => Kc3rdQuestPeriod::Daily,
            Period::Weekly => Kc3rdQuestPeriod::Weekly,
            Period::Daily3rd7th0th => Kc3rdQuestPeriod::Daily3rd7th0th,
            Period::Daily2nd8th => Kc3rdQuestPeriod::Daily2nd8th,
            Period::Monthly => Kc3rdQuestPeriod::Monthly,
            Period::Quarterly => Kc3rdQuestPeriod::Quarterly,
            Period::Annually => Kc3rdQuestPeriod::Annual,
        }
    }
}

impl TryFrom<Kc3rdQuestPeriod> for Period {
    type Error = &'static str;

    fn try_from(value: Kc3rdQuestPeriod) -> Result<Self, Self::Error> {
        match value {
            Kc3rdQuestPeriod::Oneshot => Ok(Period::Oneshot),
            Kc3rdQuestPeriod::Daily => Ok(Period::Daily),
            Kc3rdQuestPeriod::Weekly => Ok(Period::Weekly),
            Kc3rdQuestPeriod::Daily3rd7th0th => Ok(Period::Daily3rd7th0th),
            Kc3rdQuestPeriod::Daily2nd8th => Ok(Period::Daily2nd8th),
            Kc3rdQuestPeriod::Monthly => Ok(Period::Monthly),
            Kc3rdQuestPeriod::Quarterly => Ok(Period::Quarterly),
            Kc3rdQuestPeriod::Annual => Ok(Period::Annually),
            Kc3rdQuestPeriod::Unknown => Err("cannot persist Unknown quest period to DB"),
        }
    }
}

/// Trait for entities that have a timestamp and a period
pub trait HasTimestampAndPeriod {
    /// Get the start time of the quest
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc>;

    /// Get the period of the quest
    fn period(&self) -> Period;
}

impl Period {
    /// Whether quests of this period are offered on the given quest day
    /// (see [`KcTime::jst_quest_day`]). Only the two limited-day periods are
    /// tied to the day of the month.
    pub fn appears_on(self, quest_day: chrono::NaiveDate) -> bool {
        use chrono::Datelike;

        match self {
            Period::Daily3rd7th0th => matches!(quest_day.day() % 10, 0 | 3 | 7),
            Period::Daily2nd8th => matches!(quest_day.day() % 10, 2 | 8),
            _ => true,
        }
    }

    /// When a record of this period that started at `start_time` resets.
    /// `None` for one-shot quests, which never do. `yearly_month` (1 to 12) is
    /// the month a yearly quest resets in; other periods ignore it.
    pub fn reset_time(
        self,
        start_time: &chrono::DateTime<chrono::Utc>,
        yearly_month: u32,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        Some(match self {
            Period::Oneshot => return None,
            // The limited-day quests last their day only, like any daily.
            Period::Daily | Period::Daily3rd7th0th | Period::Daily2nd8th => {
                KcTime::jst_next_day_0500(start_time)
            }
            Period::Weekly => KcTime::jst_next_monday_0500(start_time),
            Period::Monthly => KcTime::jst_next_1st_day_of_the_month(start_time),
            Period::Quarterly => KcTime::jst_next_quarter_day_one_0500(start_time),
            Period::Annually => KcTime::jst_next_year_day_one_0500(start_time, yearly_month),
        })
    }
}

/// Trait for entities that can be reset
pub trait ShouldReset {
    /// Check if the quest should be reset at the given time. `yearly_month` is
    /// as in [`Period::reset_time`].
    fn should_reset(&self, now: chrono::DateTime<chrono::Utc>, yearly_month: u32) -> bool;
}

impl<T: HasTimestampAndPeriod> ShouldReset for T {
    fn should_reset(&self, now: chrono::DateTime<chrono::Utc>, yearly_month: u32) -> bool {
        self.period()
            .reset_time(&self.timestamp(), yearly_month)
            .is_some_and(|reset_time| now >= reset_time)
    }
}

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    // progress
    create_table(db, progress::Entity).await?;
    // oneshot
    create_table(db, oneshot::Entity).await?;
    // periodic
    create_table(db, periodic::Entity).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use emukc_model::prelude::Kc3rdQuestPeriod;

    use super::Period;

    #[test]
    fn try_from_all_known_variants_succeeds() {
        assert_eq!(Period::try_from(Kc3rdQuestPeriod::Oneshot).unwrap(), Period::Oneshot);
        assert_eq!(Period::try_from(Kc3rdQuestPeriod::Daily).unwrap(), Period::Daily);
        assert_eq!(Period::try_from(Kc3rdQuestPeriod::Weekly).unwrap(), Period::Weekly);
        assert_eq!(
            Period::try_from(Kc3rdQuestPeriod::Daily3rd7th0th).unwrap(),
            Period::Daily3rd7th0th
        );
        assert_eq!(Period::try_from(Kc3rdQuestPeriod::Daily2nd8th).unwrap(), Period::Daily2nd8th);
        assert_eq!(Period::try_from(Kc3rdQuestPeriod::Monthly).unwrap(), Period::Monthly);
        assert_eq!(Period::try_from(Kc3rdQuestPeriod::Quarterly).unwrap(), Period::Quarterly);
        assert_eq!(Period::try_from(Kc3rdQuestPeriod::Annual).unwrap(), Period::Annually);
    }

    #[test]
    fn try_from_unknown_returns_err() {
        assert!(Period::try_from(Kc3rdQuestPeriod::Unknown).is_err());
    }

    #[test]
    fn roundtrip_all_known_period_variants() {
        for period in [
            Period::Oneshot,
            Period::Daily,
            Period::Weekly,
            Period::Daily3rd7th0th,
            Period::Daily2nd8th,
            Period::Monthly,
            Period::Quarterly,
            Period::Annually,
        ] {
            let kc_period: Kc3rdQuestPeriod = period.into();
            let back: Period = kc_period.try_into().unwrap();
            assert_eq!(period, back);
        }
    }

    #[test]
    fn limited_day_periods_appear_by_the_last_digit_of_the_day() {
        let days = |period: Period, month| -> Vec<u32> {
            (1..=31)
                .filter_map(|day| chrono::NaiveDate::from_ymd_opt(2026, month, day))
                .filter(|day| period.appears_on(*day))
                .map(|day| chrono::Datelike::day(&day))
                .collect()
        };

        // March has a 30th and a 31st; the 31st ends in 1 and offers neither.
        assert_eq!(days(Period::Daily3rd7th0th, 3), [3, 7, 10, 13, 17, 20, 23, 27, 30]);
        assert_eq!(days(Period::Daily2nd8th, 3), [2, 8, 12, 18, 22, 28]);
        // February has no 30th.
        assert_eq!(days(Period::Daily3rd7th0th, 2), [3, 7, 10, 13, 17, 20, 23, 27]);
        assert_eq!(days(Period::Daily2nd8th, 2), [2, 8, 12, 18, 22, 28]);
        // Every other period is offered every day.
        assert_eq!(days(Period::Daily, 2).len(), 28);
        assert_eq!(days(Period::Weekly, 3).len(), 31);
    }

    #[test]
    fn limited_day_periods_reset_at_the_next_0500_like_a_daily() {
        use chrono::TimeZone;

        let jst = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        let at = |day, hour, minute| {
            jst.with_ymd_and_hms(2026, 3, day, hour, minute, 0).unwrap().with_timezone(&chrono::Utc)
        };

        for period in [Period::Daily, Period::Daily3rd7th0th, Period::Daily2nd8th] {
            assert_eq!(period.reset_time(&at(3, 23, 0), 1), Some(at(4, 5, 0)));
            assert_eq!(period.reset_time(&at(4, 4, 59), 1), Some(at(4, 5, 0)));
            assert_eq!(period.reset_time(&at(4, 5, 0), 1), Some(at(5, 5, 0)));
        }
        assert_eq!(Period::Oneshot.reset_time(&at(3, 23, 0), 1), None);
    }

    #[test]
    fn quarterly_and_yearly_periods_reset_on_the_first_of_their_month() {
        use chrono::TimeZone;

        let jst = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        let at = |year, month, day, hour| {
            jst.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap().with_timezone(&chrono::Utc)
        };

        assert_eq!(Period::Quarterly.reset_time(&at(2026, 2, 10, 12), 1), Some(at(2026, 3, 1, 5)));
        // The yearly month only moves yearly quests.
        assert_eq!(Period::Quarterly.reset_time(&at(2026, 2, 10, 12), 5), Some(at(2026, 3, 1, 5)));
        assert_eq!(Period::Annually.reset_time(&at(2026, 4, 30, 12), 5), Some(at(2026, 5, 1, 5)));
        assert_eq!(Period::Annually.reset_time(&at(2026, 4, 30, 12), 1), Some(at(2027, 1, 1, 5)));
    }
}
