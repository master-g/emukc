//! The ranking: every admiral on the server by the month's 戦果.

use emukc_internal::{
    db::{
        entity::profile,
        sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, IntoActiveModel},
    },
    time::chrono::{Datelike, FixedOffset, Utc},
};

async fn profile(context: &crate::TestContext, name: &str) -> i64 {
    let account = context.sign_up(name, "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, name).await.unwrap();
    context.start_game(&account.access_token.token, profile.profile.id).await.unwrap().profile.id
}

/// Give a profile this month's 戦果 as a bonus, or last month's.
async fn war_result(context: &crate::TestContext, pid: i64, points: i64, this_month: bool) {
    let jst = Utc::now().with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap());
    let month = i64::from(jst.year()) * 100 + i64::from(jst.month());
    let record = profile::Entity::find_by_id(pid).one(&*context.db).await.unwrap().unwrap();
    let mut am = record.into_active_model();
    am.war_result = ActiveValue::Set(points);
    am.war_result_month = ActiveValue::Set(if this_month {
        month
    } else {
        month - 1
    });
    am.update(&*context.db).await.unwrap();
}

#[tokio::test]
async fn the_ranking_orders_every_admiral_by_the_months_war_result() {
    let context = crate::TestContext::new().await;
    let mut pids = Vec::new();
    for n in 0..12 {
        let pid = profile(&context, &format!("admiral{n}")).await;
        // 0, 10, 20, ... 110; the third and the fourth tie at 20.
        war_result(
            &context,
            pid,
            if n == 3 {
                20
            } else {
                n * 10
            },
            true,
        )
        .await;
        pids.push(pid);
    }
    // Last month's 戦果 is none this month.
    let stale = profile(&context, "stale").await;
    war_result(&context, stale, 1000, false).await;

    let first = context.ranking(pids[0], 1).await.unwrap();
    assert_eq!((first.count, first.page_count, first.page), (13, 2, 1));
    let top: Vec<_> = first.entries.iter().map(|e| (e.rank, e.profile.id, e.war_result)).collect();
    assert_eq!(top[0], (1, pids[11], 110));
    // A tie goes to the earlier profile.
    assert_eq!(top[8], (9, pids[2], 20));
    assert_eq!(top[9], (10, pids[3], 20));

    // Page 0 is the asker's own: the stale admiral and the one with none are on the second.
    let own = context.ranking(stale, 0).await.unwrap();
    assert_eq!(own.page, 2);
    let rest: Vec<_> = own.entries.iter().map(|e| (e.rank, e.profile.id, e.war_result)).collect();
    assert_eq!(rest, [(11, pids[1], 10), (12, pids[0], 0), (13, stale, 0)]);
}
