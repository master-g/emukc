/// Month of a `YYYY-MM-DD` date, if it parses and is a real month.
fn release_month(release_date: Option<&str>) -> Option<i64> {
    let month = release_date?.split('-').nth(1)?.parse::<i64>().ok()?;
    (1..=12).contains(&month).then_some(month)
}

pub(super) fn extract_label_type(wiki_id: &str, release_date: Option<&str>) -> i64 {
    if wiki_id.len() < 2 {
        return 1;
    }

    // A wiki id is category letters + a period letter + a number, e.g. `By5`.
    // Only the period letter decides the label type now.
    let period: String = wiki_id.chars().filter(char::is_ascii_lowercase).collect();

    match period.as_str() {
        "d" => return 2,
        "w" => return 3,
        "m" => return 6,
        "q" => return 7,
        // A yearly quest's label is 100 + its month, so the reset month is the
        // only thing to determine. `release_date` carries it and stays correct
        // as upstream adds quests, which the two hardcoded number->month tables
        // this replaced did not: they stopped at By13 / Cy12 and had no branch
        // at all for the D / F / G categories, so 30 of the 55 yearly quests
        // ended up at 1 — the client's oneshot tab.
        "y" => {
            // The one quest whose label month is not its release month:
            // released 2020-09-17, labelled July. Established by checking the
            // old tables against release dates — 25 mapped B/C quests, 24 agree.
            if wiki_id == "By5" {
                return 107;
            }
            if let Some(month) = release_month(release_date) {
                return 100 + month;
            }
            // No release date (only By16 today). Guessing a month would be
            // inventing data; 101 at least keeps the quest on the yearly tab,
            // which `1` would not. The warning clears itself once upstream
            // fills the date in.
            warn!("no release_date for yearly quest {wiki_id}, defaulting to 101");
            return 101;
        }
        _ => {}
    }

    1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Release dates as they appear in `.data/temp/tsunkit_quests.json`.
    fn on(date: &str) -> Option<&str> {
        Some(date)
    }

    #[test]
    fn period_letter_decides_the_label_type() {
        for (wiki_id, expected) in [("Bd1", 2), ("Bw1", 3), ("Bm1", 6), ("Bq11", 7)] {
            assert_eq!(extract_label_type(wiki_id, None), expected, "{wiki_id}");
        }
    }

    #[test]
    fn yearly_quests_take_their_month_from_the_release_date() {
        // The first three agree with the tables this replaced; the rest are
        // quests those tables had no entry for.
        for (wiki_id, date, expected) in [
            ("By13", "2024-01-25", 101),
            ("By1", "2020-02-07", 102),
            ("Cy12", "2024-04-10", 104),
            ("Cy1", "2020-10-16", 110),
            ("By14", "2024-05-29", 105),
            ("By15", "2024-09-24", 109),
            ("Cy13", "2024-06-27", 106),
            ("Cy14", "2024-07-27", 107),
            ("Cy15", "2024-09-24", 109),
            ("Cy16", "2024-10-18", 110),
        ] {
            assert_eq!(extract_label_type(wiki_id, on(date)), expected, "{wiki_id}");
        }
    }

    #[test]
    fn categories_without_a_table_are_no_longer_oneshot() {
        // D / F / G yearly quests had no branch at all and all returned 1,
        // which put 23 of them on the client's oneshot tab.
        for (wiki_id, date, expected) in
            [("Dy1", "2020-02-07", 102), ("Fy10", "2023-01-20", 101), ("Gy2", "2020-11-13", 111)]
        {
            assert_eq!(extract_label_type(wiki_id, on(date)), expected, "{wiki_id}");
        }
    }

    #[test]
    fn by5_keeps_its_july_label_against_its_september_release() {
        assert_eq!(extract_label_type("By5", on("2020-09-17")), 107);
    }

    #[test]
    fn a_yearly_quest_without_a_release_date_stays_on_the_yearly_tab() {
        // By16 today. 101 is a placeholder month, but the tab is what matters:
        // `1` would file it under oneshot.
        let label = extract_label_type("By16", None);
        assert_eq!(label, 101);
        assert!((101..=112).contains(&label));
    }

    #[test]
    fn a_malformed_release_date_does_not_produce_an_out_of_range_label() {
        for date in ["2024", "2024-13-01", "2024-00-01", "not-a-date", ""] {
            let label = extract_label_type("Cy99", on(date));
            assert!((101..=112).contains(&label), "{date} gave {label}");
        }
    }

    #[test]
    fn seasonal_and_unknown_period_letters_stay_at_one() {
        // `s` is seasonal (Cs1/2/3/5/6). This repo maps Frequency::Seasonal to
        // Kc3rdQuestPeriod::Oneshot, and label_type 1 is the oneshot tab, so
        // these agree. Deliberate, not a gap.
        for wiki_id in ["Cs1", "Cs5", "A1", "X"] {
            assert_eq!(extract_label_type(wiki_id, on("2016-08-31")), 1, "{wiki_id}");
        }
    }
}
