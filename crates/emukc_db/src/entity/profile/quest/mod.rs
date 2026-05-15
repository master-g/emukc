//! Quest progress record
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

/// Trait for entities that can be reset
pub trait ShouldReset {
    /// Check if the quest should be reset.
    fn should_reset(&self) -> bool;
}

impl<T: HasTimestampAndPeriod> ShouldReset for T {
    fn should_reset(&self) -> bool {
        let start_time = &self.timestamp();
        let reset_time = match self.period() {
            Period::Oneshot => return false,
            Period::Daily => KcTime::jst_next_day_0500(start_time),
            Period::Weekly => KcTime::jst_next_monday_0500(start_time),
            Period::Daily3rd7th0th => KcTime::jst_next_370th_day_of_the_month(start_time),
            Period::Daily2nd8th => KcTime::jst_next_28th_day_of_the_month(start_time),
            Period::Monthly => KcTime::jst_next_1st_day_of_the_month(start_time),
            Period::Quarterly => KcTime::jst_next_quarter_day_one_0500(start_time),
            Period::Annually => KcTime::jst_next_year_day_one_0500(start_time),
        };

        chrono::Utc::now() > reset_time
    }
}

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    let schema = sea_orm::Schema::new(db.get_database_backend());
    // progress
    {
        let stmt = schema.create_table_from_entity(progress::Entity).if_not_exists().to_owned();
        db.execute(db.get_database_backend().build(&stmt)).await?;
    }
    // oneshot
    {
        let stmt = schema.create_table_from_entity(oneshot::Entity).if_not_exists().to_owned();
        db.execute(db.get_database_backend().build(&stmt)).await?;
    }
    // periodic
    {
        let stmt = schema.create_table_from_entity(periodic::Entity).if_not_exists().to_owned();
        db.execute(db.get_database_backend().build(&stmt)).await?;
    }

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
}
