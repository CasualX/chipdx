use std::{env, fs};

fn play(level: &chipty::LevelDto, replay: &chipty::ReplayDto) -> bool {
	let seed: u64 = u64::from_str_radix(&replay.seed, 16).unwrap();
	let inputs = chipty::decode(&replay.inputs);

	let mut game = chipcore::GameState::default();
	game.parse(level, chipcore::RngSeed::Manual(seed));

	for input in inputs {
		if game.is_game_over() {
			break;
		}
		game.tick(&chipcore::Input::decode(input));
	}

	let events = game.events.take();
	let game_over_reason = events.iter().find_map(|event| match event {
		&chipcore::GameEvent::GameOver { reason } => Some(reason),
		_ => None,
	});
	if game_over_reason == Some(chipcore::GameOverReason::LevelComplete) {
		return true;
	}

	eprintln!(" - FAILED: expected LevelComplete, got {game_over_reason:?}");
	return false;
}

#[test]
fn pocket_wonders() {
	let current_dir = env::current_dir().unwrap();
	let levels_dir = current_dir.parent().unwrap().parent().unwrap().join("levelsets/pocket-wonders/lv");
	let replays_dir = current_dir.join("tests/replays/pocket-wonders");

	let mut paths: Vec<_> = fs::read_dir(replays_dir).unwrap()
		.map(|entry| entry.unwrap().path())
		.filter(|path| path.extension().is_some_and(|ext| ext == "json"))
		.collect();
	paths.sort();

	let mut fail_count = 0;
	let mut tested_count = 0;
	for replay_path in paths {
		let name = replay_path.file_stem().unwrap().to_str().unwrap();
		let level_name = name.split('-').next().unwrap();
		let level_path = levels_dir.join(format!("{level_name}.json"));
		let level_content = fs::read_to_string(level_path).unwrap();
		let level: chipty::LevelDto = serde_json::from_str(&level_content).unwrap();
		let replay_content = fs::read_to_string(&replay_path).unwrap();
		let replay: chipty::ReplayDto = serde_json::from_str(&replay_content).unwrap();

		eprintln!("Playing: {name}: \x1b[32m{}\x1b[m", level.name);
		tested_count += 1;
		fail_count += !play(&level, &replay) as usize;
	}

	assert!(tested_count > 0, "No Pocket Wonders replays found");
	eprintln!("Played {tested_count} replays");
	if fail_count > 0 {
		panic!("{} replay(s) failed", fail_count);
	}
}
