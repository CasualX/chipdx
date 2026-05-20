use std::{fs, path};

fn main() {
	let matches = clap::command!()
		.about("Unpack a Chip's Challenge DAT file into a JSON levelset structure")
		.arg(clap::arg!(<INPUT> "Path to the input .dat file (MS/Steam style DAT)")
			.value_parser(clap::value_parser!(path::PathBuf)))
		.arg(clap::arg!([OUT_DIR] "Directory to write the extracted levelset (created if missing, defaults to the input's directory)")
			.value_parser(clap::value_parser!(path::PathBuf)))
		.arg(clap::arg!(-e --encoding [ENCODING] "Text encoding [default: windows1252]")
			.value_parser(["utf8", "latin1", "windows1252"]))
		.arg(clap::arg!(--direct "Keeps levels embedded within index instead of separate files")
			.action(clap::ArgAction::SetTrue))
		.get_matches();

	let input = matches.get_one::<path::PathBuf>("INPUT").unwrap().as_path();
	let out_dir = matches
		.get_one::<path::PathBuf>("OUT_DIR").map(path::PathBuf::as_path)
		.or_else(|| input.parent())
		.unwrap_or_else(|| path::Path::new("."));
	let encoding = match matches.get_one::<String>("encoding").map(String::as_str).unwrap_or("windows1252") {
		"utf8" => chipdat::Encoding::Utf8,
		"latin1" => chipdat::Encoding::Latin1,
		"windows1252" => chipdat::Encoding::Windows1252,
		_ => chipdat::Encoding::Windows1252,
	};
	let direct = matches.get_flag("direct");

	// Parse the DAT file
	let opts = chipdat::Options { encoding };
	let dat = chipdat::read(input, &opts).expect("Failed to read DAT file");

	// Convert to levelset structure
	let title = input.file_stem().unwrap().to_str().unwrap();
	let mut levelset = chipdat::convert(&dat, title.to_string());

	let levelset_path = out_dir.join(title);
	let levelset_index = levelset_path.join("index.json");
	let _ = fs::create_dir(&levelset_path);

	let index = if direct {
		// Keep levels embedded directly in index
		serde_json::to_string(&levelset)
	}
	else {
		// Write levels as separate files
		let levels_base_path = levelset_path.join("lv");
		let _ = fs::create_dir(&levels_base_path);

		for (index, level_ref) in levelset.levels.iter_mut().enumerate() {
			if let chipty::LevelRef::Direct(level) = level_ref {
				let level_path = levels_base_path.join(format!("level{}.json", index + 1));
				let level_string = serde_json::to_string(level).unwrap();
				eprintln!("Writing level {}", level_path.display());
				fs::write(&level_path, level_string).expect("Failed to write level file");

				// Update level ref to be indirect
				let level_relative_path = level_path.strip_prefix(&levelset_path).unwrap();
				*level_ref = chipty::LevelRef::Indirect(level_relative_path.to_string_lossy().to_string());
			}
		}

		serde_json::to_string_pretty(&levelset)
	}.unwrap();

	eprintln!("Writing levelset {}", levelset_index.display());
	fs::write(&levelset_index, index).expect("Failed to write levelset index");
}
