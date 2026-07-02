use std::{fs, path};

fn main() {
	let matches = clap::command!()
		.about("Create a playable level share link from local file")
		.arg(clap::arg!(--json <JSON_PATH> "Path to a level JSON file").value_parser(clap::value_parser!(path::PathBuf)))
		.arg(clap::arg!(--paks <PAKS_PATH> "Path to a packed levelset .paks").value_parser(clap::value_parser!(path::PathBuf)))
		.arg(clap::arg!(--dat <DAT_PATH> "Path to a levelset .dat").value_parser(clap::value_parser!(path::PathBuf)))
		.arg(clap::arg!(-k --key [KEY] "Packed levelset encryption key"))
		.arg(clap::arg!([LEVEL] "Level number (1-based) or case-insensitive level name substring"))
		.get_matches();

	let json = matches.get_one::<path::PathBuf>("json");
	let paks = matches.get_one::<path::PathBuf>("paks");
	let dat = matches.get_one::<path::PathBuf>("dat");
	let selector = matches.get_one::<String>("LEVEL");

	let mut level = match (json, paks, dat) {
		(Some(path), None, None) => {
			if selector.is_some() {
				fail("LEVEL is only used with --paks or --dat");
			}
			load_level_json(path)
		}
		(None, Some(path), None) => {
			let Some(selector) = selector else {
				fail("missing LEVEL number or name for --paks");
			};
			let key = matches.get_one::<String>("key")
				.map(|s| paks::parse_key(s).unwrap_or_else(|err| panic!("invalid key: {err}")))
				.unwrap_or_default();
			load_level_paks(path, &key, selector)
		}
		(None, None, Some(path)) => {
			let Some(selector) = selector else {
				fail("missing LEVEL number or name for --dat");
			};
			load_level_dat(path, selector)
		}
		(None, None, None) => fail("missing --json PATH, --paks PATH, or --dat PATH"),
		_ => fail("use exactly one of --json, --paks, or --dat"),
	};

	// Strip the replays from the level before encoding, as they can be large and aren't needed for playing the level
	level.replays = None;
	let level_json = serde_json::to_string(&level).expect("Failed to serialize level JSON");

	let encoded = chipty::encode_level(level_json.as_bytes());

	let decoded = chipty::decode_level(&encoded);
	assert_eq!(level_json.as_bytes(), decoded);

	println!("https://casualhacks.net/chipdx/#?levelc={}", encoded);
}

fn fail(message: &str) -> ! {
	eprintln!("levelencode: {message}");
	std::process::exit(2);
}

fn load_level_json(path: &path::Path) -> chipty::LevelDto {
	let level_json = match fs::read(path) {
		Ok(level_json) => level_json,
		Err(err) => {
			eprintln!("Failed to read {}: {}", path.display(), err);
			std::process::exit(1);
		}
	};

	match serde_json::from_slice(&level_json) {
		Ok(level) => level,
		Err(err) => {
			eprintln!("Failed to parse level JSON: {}", err);
			std::process::exit(1);
		}
	}
}

fn load_level_paks(path: &path::Path, key: &paks::Key, selector: &str) -> chipty::LevelDto {
	let reader = paks::FileReader::open(path, key).unwrap_or_else(|err| panic!("open {}: {err}", path.display()));
	let desc = reader.find_file(b"index.json").unwrap_or_else(|| panic!("{} does not contain index.json", path.display()));
	let data = reader.read_data(desc, key).unwrap_or_else(|err| panic!("read index.json from {}: {err}", path.display()));
	let index_data = chipty::decompress(&data);
	let levelset: chipty::LevelSetDto = serde_json::from_slice(&index_data).unwrap_or_else(|err| panic!("parse index.json from {}: {err}", path.display()));
	select_level(&levelset, path, selector)
}

fn load_level_dat(path: &path::Path, selector: &str) -> chipty::LevelDto {
	let opts = chipdat::Options {
		encoding: chipdat::Encoding::Windows1252,
	};
	let dat = chipdat::read(path, &opts).unwrap_or_else(|err| panic!("read {}: {err:?}", path.display()));
	let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("levelset").to_string();
	let levelset = chipdat::convert(&dat, title);
	select_level(&levelset, path, selector)
}

fn select_level(levelset: &chipty::LevelSetDto, path: &path::Path, selector: &str) -> chipty::LevelDto {
	if let Ok(number) = selector.parse::<usize>() {
		if number == 0 || number > levelset.levels.len() {
			panic!("level number {number} is outside 1..={}", levelset.levels.len());
		}
		return level_from_ref(&levelset.levels[number - 1], number);
	}

	let selector = selector.to_lowercase();
	for (index, level_ref) in levelset.levels.iter().enumerate() {
		let level = level_from_ref(level_ref, index + 1);
		if level.name.to_lowercase().contains(&selector) {
			return level;
		}
	}

	panic!("level matching {selector:?} not found in {}", path.display());
}

fn level_from_ref(level_ref: &chipty::LevelRef, level_number: usize) -> chipty::LevelDto {
	match level_ref {
		chipty::LevelRef::Direct(level) => level.clone(),
		chipty::LevelRef::Indirect(_) => panic!("level {level_number} is indirect; levelencode needs embedded direct levels"),
	}
}
