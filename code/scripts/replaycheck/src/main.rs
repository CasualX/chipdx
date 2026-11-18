use std::{collections::HashMap, fs, path::{Path, PathBuf}, process::ExitCode};

fn load_levels(path: &Path) -> Result<HashMap<String, chipty::LevelDto>, String> {
	let index = if path.is_dir() { path.join("index.json") } else { path.to_owned() };
	let content = fs::read_to_string(&index).map_err(|err| format!("{}: {err}", index.display()))?;
	let levelset: chipty::LevelSetDto = serde_json::from_str(&content)
		.map_err(|err| format!("{}: {err}", index.display()))?;
	let mut levels = HashMap::new();
	for entry in levelset.levels {
		let level = match entry {
			chipty::LevelRef::Direct(level) => level,
			chipty::LevelRef::Indirect(relative) => {
				let path = index.parent().unwrap().join(relative);
				let content = fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
				serde_json::from_str::<chipty::LevelDto>(&content)
					.map_err(|err| format!("{}: {err}", path.display()))?
			},
		};
		let name = level.name.clone();
		if levels.insert(name.clone(), level).is_some() {
			return Err(format!("Duplicate level name {name:?}; cannot match replays unambiguously"));
		}
	}
	Ok(levels)
}

fn play(level: &chipty::LevelDto, replay: &chipty::ReplayDto) -> Result<(), String> {
	let seed = u64::from_str_radix(&replay.seed, 16).map_err(|err| format!("Invalid seed: {err}"))?;
	let inputs = chipty::try_decode(&replay.inputs).ok_or("Invalid encoded replay inputs")?;
	let mut game = chipcore::GameState::default();
	game.parse(level, chipcore::RngSeed::Manual(seed));
	for input in inputs {
		game.tick(&chipcore::Input::decode(input));
		for event in game.events.take() {
			if let chipcore::GameEvent::GameOver { reason } = event {
				return if reason == chipcore::GameOverReason::LevelComplete {
					Ok(())
				} else {
					Err(format!("Expected LevelComplete, got {reason:?}"))
				};
			}
		}
		if game.is_game_over() {
			break;
		}
	}
	Err("Replay ended without GameOver(LevelComplete)".into())
}

fn check(levelset: &Path, replays: &Path) -> Result<bool, String> {
	let levels = load_levels(levelset)?;
	let mut paths = fs::read_dir(replays).map_err(|err| format!("{}: {err}", replays.display()))?
		.map(|entry| entry.map(|entry| entry.path()))
		.collect::<Result<Vec<_>, _>>().map_err(|err| err.to_string())?;
	paths.retain(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "json"));
	paths.sort();
	let mut passed = 0;
	let mut failed = 0;
	let mut skipped = 0;
	for path in paths {
		let result: Result<bool, String> = (|| {
			let content = fs::read_to_string(&path).map_err(|err| err.to_string())?;
			let replay: chipty::ReplayDto = serde_json::from_str(&content).map_err(|err| err.to_string())?;
			let Some(name) = replay.level_name.as_deref() else {
				eprintln!("WARNING: {}: no level_name; skipped", path.display());
				return Ok(false);
			};
			let Some(level) = levels.get(name) else {
				eprintln!("WARNING: {}: level {name:?} not found; skipped", path.display());
				return Ok(false);
			};
			play(level, &replay)?;
			println!("PASS: {}: {name}", path.display());
			Ok(true)
		})();
		match result {
			Ok(true) => passed += 1,
			Ok(false) => skipped += 1,
			Err(err) => {
				eprintln!("FAIL: {}: {err}", path.display());
				failed += 1;
			},
		}
	}
	println!("{passed} passed, {failed} failed, {skipped} skipped");
	if passed + failed == 0 {
		eprintln!("WARNING: No matching replays were checked");
	}
	Ok(failed == 0)
}

fn main() -> ExitCode {
	let matches = clap::command!()
		.about("Check replay completion, matching each replay's level_name to a level in the set")
		.arg(clap::arg!(<LEVELSET> "Levelset directory or index.json").value_parser(clap::value_parser!(PathBuf)))
		.arg(clap::arg!(<REPLAYS> "Directory containing replay JSON files").value_parser(clap::value_parser!(PathBuf)))
		.get_matches();
	match check(matches.get_one::<PathBuf>("LEVELSET").unwrap(), matches.get_one::<PathBuf>("REPLAYS").unwrap()) {
		Ok(true) => ExitCode::SUCCESS,
		Ok(false) => ExitCode::FAILURE,
		Err(err) => {
			eprintln!("ERROR: {err}");
			ExitCode::FAILURE
		},
	}
}
