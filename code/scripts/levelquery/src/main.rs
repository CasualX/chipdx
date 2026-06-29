use std::collections::HashSet;
use std::{fs, path};
use cvmath::*;

struct LevelScore {
	levelset_title: String,
	level_number: usize,
	level_name: String,
	score: f64,
}

fn main() {
	let matches = clap::command!()
		.about("Find levels with certain properties")
		.arg(clap::arg!(<PATH> "Path(s) to levelsets").value_parser(clap::value_parser!(path::PathBuf)).num_args(1..))
		.arg(clap::arg!(--score [EXPR] "Scoring function").required(true))
		.arg(clap::arg!(--"min-score" [MIN_SCORE] "Minimum score required to display a level").required(false).value_parser(clap::value_parser!(f64)))
		.arg(clap::arg!(--top [N] "Show only the top N scoring levels").required(false).value_parser(clap::value_parser!(usize)))
		.arg(clap::arg!(--bottom [N] "Show only the bottom N scoring levels").required(false).value_parser(clap::value_parser!(usize)))
		.get_matches();

	let paths: Vec<_> = matches.get_many::<path::PathBuf>("PATH").expect("At least one PATH must be provided").collect();
	let score_expr = matches.get_one::<String>("score").unwrap();
	let min_score: f64 = *matches.get_one::<f64>("min-score").unwrap_or(&0.0);
	let top_n: Option<usize> = matches.get_one::<usize>("top").cloned();
	let bottom_n: Option<usize> = matches.get_one::<usize>("bottom").cloned();
	if top_n.is_some() && bottom_n.is_some() {
		eprintln!("--top and --bottom are mutually exclusive; please use only one");
		std::process::exit(1);
	}

	let mut scores = vec![];

	for base_path in paths {
		// Load the levelset
		let levelset = match load_levelset(base_path) {
			Some(value) => value,
			None => continue,
		};

		// Process each level in the levelset
		for (idx, level_ref) in levelset.levels.into_iter().enumerate() {
			// Load the level from the reference
			let level = match level_from_ref(base_path, level_ref) {
				Some(value) => value,
				None => continue,
			};

			// Score the level according to the scoring expression
			let score = score_level(&level, &score_expr);

			// Apply the minimum score filter
			if score > min_score {
				scores.push(LevelScore {
					levelset_title: levelset.title.clone(),
					level_number: idx + 1,
					level_name: level.name.clone(),
					score,
				});
			}
		}
	}

	// Sort according to the requested slice: top (desc), bottom (asc), or default (desc).
	match (top_n, bottom_n) {
		(Some(_), None) | (None, None) => {
			// Top N or default: highest scores first.
			scores.sort_by(|a, b| b.score.total_cmp(&a.score));
		}
		(None, Some(_)) => {
			// Bottom N: lowest scores first.
			scores.sort_by(|a, b| a.score.total_cmp(&b.score));
		}
		_ => unreachable!(),
	}

	// Determine how many entries to show, respecting the requested limit but
	// extending it to include all ties with the cutoff score to avoid arbitrary truncation.
	let mut limit = scores.len();
	if let Some(requested) = top_n.or(bottom_n) {
		if !scores.is_empty() {
			limit = requested.min(scores.len());
			if limit < scores.len() {
				let cutoff_score = scores[limit - 1].score;
				while limit < scores.len() && scores[limit].score == cutoff_score {
					limit += 1;
				}
			}
		}
	}

	print_table(&scores[..limit]);
}

fn format_score(score: f64) -> String {
	let mut s = format!("{score:.3}");
	if s.contains('.') {
		while s.ends_with('0') {
			s.pop();
		}
		if s.ends_with('.') {
			s.pop();
		}
	}
	return s;
}

fn print_table(rows: &[LevelScore]) {
	if rows.is_empty() {
		return;
	}

	let header_levelset = "Levelset";
	let header_level = "Lv";
	let header_score = "Score";
	let header_name = "Name";

	let mut levelset_width = header_levelset.len();
	let mut level_width = header_level.len();
	let mut score_width = header_score.len();
	let mut name_width = header_name.len();

	let mut rendered: Vec<(&str, String, String, &str)> = Vec::with_capacity(rows.len());
	for row in rows {
		let level = row.level_number.to_string();
		let score = format_score(row.score);
		rendered.push((&row.levelset_title, level, score, &row.level_name));
	}

	for (levelset, level, score, name) in &rendered {
		levelset_width = levelset_width.max(levelset.len());
		level_width = level_width.max(level.len());
		score_width = score_width.max(score.len());
		name_width = name_width.max(name.len());
	}

	println!(
		"{:<levelset_width$}  {:>level_width$}  {:>score_width$}  {:<name_width$}",
		header_levelset,
		header_level,
		header_score,
		header_name,
		levelset_width = levelset_width,
		level_width = level_width,
		score_width = score_width,
		name_width = name_width
	);
	println!(
		"{}  {}  {}  {}",
		"-".repeat(levelset_width),
		"-".repeat(level_width),
		"-".repeat(score_width),
		"-".repeat(name_width)
	);

	for (levelset, level, score, name) in rendered {
		println!(
			"{:<levelset_width$}  {:>level_width$}  {:>score_width$}  {:<name_width$}",
			levelset,
			level,
			score,
			name,
			levelset_width = levelset_width,
			level_width = level_width,
			score_width = score_width,
			name_width = name_width
		);
	}
}

fn load_levelset(base_path: &path::PathBuf) -> Option<chipty::LevelSetDto> {
	if base_path.is_dir() {
		let index_path = base_path.join("index.json");
		let Ok(index_data) = fs::read_to_string(&index_path)
		else {
			eprintln!("Failed to read levelset index at {}", index_path.display());
			return None;
		};

		let levelset: chipty::LevelSetDto = match serde_json::from_str(&index_data) {
			Ok(ls) => ls,
			Err(err) => {
				eprintln!("Invalid levelset index at {}: {}", index_path.display(), err);
				return None;
			}
		};

		return Some(levelset);
	}
	else if base_path.is_file() {
		let ext = base_path.extension().and_then(|s| s.to_str());
		if ext == Some("dat") {
			let opts = chipdat::Options {
				encoding: chipdat::Encoding::Windows1252,
			};
			if let Ok(data) = chipdat::read(base_path, &opts) {
				let title = base_path.file_stem().and_then(|s| s.to_str()).unwrap_or("levelset").to_string();
				return Some(chipdat::convert(&data, title));
			}
		}
		else {
			eprintln!("Only .dat levelset files are supported at the moment");
		}
	}
	else {
		eprintln!("Path {} is neither a directory nor a file", base_path.display());
	}
	return None;
}

fn level_from_ref(base_path: &path::PathBuf, level_ref: chipty::LevelRef) -> Option<chipty::LevelDto> {
	Some(match level_ref {
		chipty::LevelRef::Direct(level) => level,
		chipty::LevelRef::Indirect(rel_path) => {
			let level_path = base_path.join(&rel_path);
			let Ok(content) = fs::read_to_string(&level_path)
			else {
				eprintln!("Failed to read level at {}", level_path.display());
				return None;
			};
			let level: chipty::LevelDto = match serde_json::from_str(&content) {
				Ok(level) => level,
				Err(err) => {
					eprintln!("Invalid level JSON at {}: {}", level_path.display(), err);
					return None;
				}
			};
			level
		}
	})
}

fn score_level(level: &chipty::LevelDto, score_expr: &str) -> f64 {
	let env = ExprContext {
		basic: pupil::BasicEnv::default(),
		level,
	};
	pupil::eval(&env, score_expr).expect("Failed to evaluate score expression")
}

struct ExprContext<'a> {
	basic: pupil::BasicEnv,
	level: &'a chipty::LevelDto,
}

impl<'a> pupil::Env for ExprContext<'a> {
	fn function(&self, name: &str) -> Result<pupil::Function, pupil::ErrorKind> {
		self.basic.function(name)
	}

	fn value(&self, name: &str) -> Result<pupil::Value, pupil::ErrorKind> {
		if let Ok(value) = self.basic.value(name) {
			return Ok(value);
		}

		match name {
			"Map.Width" => Ok(self.level.map.width as f64),
			"Map.Height" => Ok(self.level.map.height as f64),
			"Map.Area" => {
				let width = self.level.map.width as f64;
				let height = self.level.map.height as f64;
				Ok(width * height)
			}
			"Map.UniqueTerrainTypes" => {
				Ok(self.level.map.legend.len() as i32 as f64)
			}
			"NumEntities" => Ok(self.level.entities.len() as f64),
			"Monster" => count_entities(self.level, &[chipty::EntityKind::Bug, chipty::EntityKind::FireBall, chipty::EntityKind::PinkBall, chipty::EntityKind::Tank, chipty::EntityKind::Glider, chipty::EntityKind::Teeth, chipty::EntityKind::Walker, chipty::EntityKind::Blob, chipty::EntityKind::Paramecium]),
			"Key" => count_entities(self.level, &[chipty::EntityKind::BlueKey, chipty::EntityKind::RedKey, chipty::EntityKind::GreenKey, chipty::EntityKind::YellowKey]),
			"CountUniqueEntities" => {
				let mut kinds = HashSet::new();
				for entity in &self.level.entities {
					kinds.insert(entity.kind);
				}
				Ok(kinds.len() as f64)
			}
			"Ice" => count_terrain(self.level, &[chipty::Terrain::Ice, chipty::Terrain::IceNE, chipty::Terrain::IceNW, chipty::Terrain::IceSE, chipty::Terrain::IceSW]),
			"ForceFloor" => count_terrain(self.level, &[chipty::Terrain::ForceE, chipty::Terrain::ForceN, chipty::Terrain::ForceS, chipty::Terrain::ForceW, chipty::Terrain::ForceRandom]),
			"CloneBlock" => count_terrain(self.level, &[chipty::Terrain::CloneBlockE, chipty::Terrain::CloneBlockN, chipty::Terrain::CloneBlockS, chipty::Terrain::CloneBlockW]),
			"ThinWall" => count_terrain(self.level, &[chipty::Terrain::ThinWallE, chipty::Terrain::ThinWallN, chipty::Terrain::ThinWallS, chipty::Terrain::ThinWallW, chipty::Terrain::ThinWallNW, chipty::Terrain::ThinWallNE, chipty::Terrain::ThinWallSW, chipty::Terrain::ThinWallSE, chipty::Terrain::ThinWallH, chipty::Terrain::ThinWallV]),
			_ => {
				if let Some(pattern) = name.strip_prefix("PatternSeq:") {
					return count_patterns_seq(self.level, pattern);
				}
				if let Some(pattern) = name.strip_prefix("PatternTile:") {
					return count_patterns_tile(self.level, pattern);
				}
				let mut count = 0;
				let mut success = false;
				if let Ok(entity_kind) = name.parse::<chipty::EntityKind>() {
					success = true;
					for entity in &self.level.entities {
						if entity.kind == entity_kind {
							count += 1;
						}
					}
				}
				if let Ok(terrain) = name.parse::<chipty::Terrain>() {
					success = true;
					if let Some(index) = self.level.map.legend.iter().position(|t| *t == terrain) {
						count += self.level.map.data.iter().filter(|&&t_idx| t_idx as usize == index).count();
					}
				}
				if !success {
					return Err(pupil::ErrorKind::NameNotFound);
				}
				Ok(count as f64)
			}
		}
	}

	fn set_value(&mut self, name: &str, value: pupil::Value) -> Result<(), pupil::ErrorKind> {
		self.basic.set_value(name, value)
	}
}

fn count_terrain(level: &chipty::LevelDto, terrains: &[chipty::Terrain]) -> Result<f64, pupil::ErrorKind> {
	let mut mask = 0u64;
	for &terrain in terrains {
		if let Some(index) = level.map.legend.iter().position(|&value| value == terrain) {
			mask |= 1 << index;
		}
	}
	let count = level.map.data.iter().filter(|&&t_idx| (mask & (1 << t_idx)) != 0).count() as i32;
	Ok(count as f64)
}

fn count_entities(level: &chipty::LevelDto, kinds: &[chipty::EntityKind]) -> Result<f64, pupil::ErrorKind> {
	let mut mask = 0u64;
	for &kind in kinds {
		mask |= 1 << (kind as u64);
	}
	let count = level.entities.iter().filter(|entity| (mask & (1 << (entity.kind as u64))) != 0).count() as i32;
	Ok(count as f64)
}

enum Thing {
	Entity(chipty::EntityKind),
	Terrain(chipty::Terrain),
}

fn parse_pattern(level: &chipty::LevelDto, pattern: &str) -> (HashSet<(i32, i32, chipty::EntityKind)>, Vec<Option<Thing>>) {
	let mut entity_lookup = HashSet::new();
	let mut entity_lookup_created = false;

	let things: Vec<Option<Thing>> = pattern.split(":").map(|s| {
		if let Ok(entity_kind) = s.parse::<chipty::EntityKind>() {
			if !entity_lookup_created {
				entity_lookup_created = true;
				for entity in &level.entities {
					entity_lookup.insert((entity.pos.x, entity.pos.y, entity.kind));
				}
			}
			Some(Thing::Entity(entity_kind))
		}
		else if let Ok(terrain) = s.parse::<chipty::Terrain>() {
			Some(Thing::Terrain(terrain))
		}
		else if s == "*" {
			None
		}
		else {
			panic!("Unknown pattern component: {}", s);
		}
	}).collect();

	(entity_lookup, things)
}

fn pattern_matches(level: &chipty::LevelDto, start: Vec2<i32>, dir: Option<chipty::Compass>, things: &Vec<Option<Thing>>, entity_lookup: &HashSet<(i32, i32, chipty::EntityKind)>) -> bool {
	for (step, thing) in things.iter().enumerate() {
		let pos = match dir {
			Some(direction) => start + direction.to_vec() * (step as i32),
			None => start,
		};

		// Abort if we step outside the map
		if pos.x < 0 || pos.y < 0 || pos.x >= level.map.width || pos.y >= level.map.height {
			return false;
		}

		// Wildcard matches anything at this position
		let Some(thing) = thing else { continue };

		let idx = (pos.y * level.map.width + pos.x) as usize;
		let tile_index = level.map.data[idx] as usize;
		let terrain = level.map.legend[tile_index];

		match thing {
			&Thing::Terrain(expected_terrain) => {
				if terrain != expected_terrain {
					return false;
				}
			}
			&Thing::Entity(expected_kind) => {
				if !entity_lookup.contains(&(pos.x, pos.y, expected_kind)) {
					return false;
				}
			}
		}
	}

	true
}

fn count_patterns_seq(level: &chipty::LevelDto, pattern: &str) -> Result<f64, pupil::ErrorKind> {
	let (entity_lookup, things) = parse_pattern(level, pattern);

	// Empty patterns always yield 0
	if things.is_empty() {
		return Ok(0.0);
	}

	// Look for this pattern from left to right, right to left, top to bottom, bottom to top.
	let mut count = 0;
	for y in 0..level.map.height {
		for x in 0..level.map.width {
			let pos = Vec2 { x, y };
			if pattern_matches(level, pos, Some(chipty::Compass::Left), &things, &entity_lookup)
				|| pattern_matches(level, pos, Some(chipty::Compass::Right), &things, &entity_lookup)
				|| pattern_matches(level, pos, Some(chipty::Compass::Down), &things, &entity_lookup)
				|| pattern_matches(level, pos, Some(chipty::Compass::Up), &things, &entity_lookup)
			{
				count += 1;
			}
		}
	}
	Ok(count as f64)
}

fn count_patterns_tile(level: &chipty::LevelDto, pattern: &str) -> Result<f64, pupil::ErrorKind> {
	let (entity_lookup, things) = parse_pattern(level, pattern);

	// Empty patterns always yield 0
	if things.is_empty() {
		return Ok(0.0);
	}

	let mut count = 0;
	for y in 0..level.map.height {
		for x in 0..level.map.width {
			let pos = Vec2 { x, y };
			if pattern_matches(level, pos, None, &things, &entity_lookup) {
				count += 1;
			}
		}
	}

	Ok(count as f64)
}
