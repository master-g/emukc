//! dump fs tree

use std::path::Path;

fn main() {
	// get args
	let args = std::env::args().collect::<Vec<String>>();
	if args.len() != 2 {
		eprintln!("Usage: dump_tree <dir>");
		std::process::exit(1);
	}
	let root = &args[1];
	let tree = dump_tree(root);

	let list = tree.into_iter().map(|p| p.replace(root, "")).collect::<Vec<String>>();

	for item in list {
		println!("\"{}\",", item);
	}
}

fn dump_tree(root: impl AsRef<Path>) -> Vec<String> {
	let mut tree = Vec::new();
	let mut stack = vec![(root.as_ref().to_owned(), 0)];

	let mut head = String::new();

	while let Some((path, depth)) = stack.pop() {
		head.push_str(&format!("{}{}\n", "  ".repeat(depth), path.display()));

		if let Ok(entries) = path.read_dir() {
			for entry in entries {
				if let Ok(entry) = entry {
					stack.push((entry.path(), depth + 1));

					if entry.file_type().unwrap().is_file() {
						tree.push(entry.path().to_string_lossy().into_owned());
					}
				}
			}
		}
	}

	tree
}
