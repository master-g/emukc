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
