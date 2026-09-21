pub(super) fn extract_label_type(wiki_id: &str) -> i64 {
    if wiki_id.len() < 2 {
        return 1;
    }

    let mut category: String = String::new();
    let mut period: String = String::new();
    let mut number: String = String::new();

    for c in wiki_id.chars() {
        if c.is_ascii_alphabetic() {
            if c.is_ascii_uppercase() {
                category.push(c);
            } else {
                period.push(c);
            }
        } else if c.is_ascii_digit() {
            number.push(c);
        }
    }

    let num = number.parse::<i64>().unwrap_or_else(|_| {
        error!("Failed to parse number from wiki_id: {}", wiki_id);
        1
    });

    match period.as_str() {
        "d" => return 2,
        "w" => return 3,
        "m" => return 6,
        "q" => return 7,
        "y" => match category.as_str() {
            "B" => {
                // (label_type, [quest_number])
                // label_type, 100 + month, eg. 101 for January, 102 for February etc.
                let table = [
                    (101, vec![13]),
                    (102, vec![1, 2]),
                    (103, vec![3, 4]),
                    (105, vec![11, 12]),
                    (106, vec![6, 7, 8, 9, 10]),
                    (107, vec![5]),
                ];
                if let Some(t) = table.iter().find(|(_, l)| l.contains(&num)).map(|(t, _)| *t) {
                    return t;
                }
                error!("Failed to find label type for wiki_id: {}", wiki_id);
                return 1;
            }
            "C" => {
                let table = [
                    (102, vec![3]),
                    (103, vec![4]),
                    (104, vec![10, 12]),
                    (105, vec![8]),
                    (106, vec![5, 9]),
                    (107, vec![6, 11]),
                    (110, vec![1, 2, 7]),
                ];
                if let Some(t) = table.iter().find(|(_, l)| l.contains(&num)).map(|(t, _)| *t) {
                    return t;
                }
                error!("Failed to find label type for wiki_id: {}", wiki_id);
                return 1;
            }
            _ => {}
        },
        _ => {}
    }

    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_letter_decides_the_label_type() {
        for (wiki_id, expected) in [("Bd1", 2), ("Bw1", 3), ("Bm1", 6), ("Bq11", 7)] {
            assert_eq!(extract_label_type(wiki_id), expected, "{wiki_id}");
        }
    }

    #[test]
    fn yearly_b_quests_map_to_their_month() {
        for (wiki_id, expected) in [("By13", 101), ("By1", 102), ("By3", 103), ("By5", 107)] {
            assert_eq!(extract_label_type(wiki_id), expected, "{wiki_id}");
        }
    }

    #[test]
    fn yearly_c_quests_map_to_their_month() {
        for (wiki_id, expected) in [("Cy3", 102), ("Cy4", 103), ("Cy10", 104), ("Cy1", 110)] {
            assert_eq!(extract_label_type(wiki_id), expected, "{wiki_id}");
        }
    }

    #[test]
    fn yearly_quests_missing_from_the_tables_fall_back_to_one() {
        // 当前行为，计划 011 会翻转：这些是年任务，不应落到 label_type 1
        for wiki_id in ["By14", "By15", "By16", "Cy13", "Cy14", "Cy15", "Cy16"] {
            assert_eq!(extract_label_type(wiki_id), 1, "{wiki_id}");
        }
    }

    #[test]
    fn unknown_period_letters_and_short_ids_fall_back_to_one() {
        // `s` 有 5 个真实 wiki_id（Cs1/2/3/5/6），但 match 里没有这个分支
        for wiki_id in ["Cs1", "Cs5", "A1", "X"] {
            assert_eq!(extract_label_type(wiki_id), 1, "{wiki_id}");
        }
    }
}
