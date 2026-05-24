use std::{fs, path};

fn main() {
	let matches = clap::command!()
		.about("Compress and base64-encode a level JSON file for the html shell")
		.arg(
			clap::arg!(<PATH> "Path to a level JSON file")
				.value_parser(clap::value_parser!(path::PathBuf)),
		)
		.get_matches();

	let path = matches.get_one::<path::PathBuf>("PATH").expect("PATH is required");
	let level_json = match fs::read(path) {
		Ok(level_json) => level_json,
		Err(err) => {
			eprintln!("Failed to read {}: {}", path.display(), err);
			std::process::exit(1);
		}
	};

	// Strip the replays from the level before encoding, as they can be large and aren't needed for playing the level
	let mut level: chipty::LevelDto = match serde_json::from_slice(&level_json) {
		Ok(level) => level,
		Err(err) => {
			eprintln!("Failed to parse level JSON: {}", err);
			std::process::exit(1);
		}
	};
	level.replays = None;
	let level_json = serde_json::to_string(&level).expect("Failed to serialize level JSON");

	let encoded = chipty::encode_level(level_json.as_bytes());

	// let decoded = chipty::decode_level(&encoded);
	// assert_eq!(level_json, decoded);

	println!("https://casualhacks.net/chipdx/?levelc={}", encoded);
}
