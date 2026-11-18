use std::collections::BTreeMap;

use super::*;

use chipty::{SaveFileDto, HighScoresDto};

pub struct Scores {
	pub ticks: i32,
	pub steps: i32,
	// pub attempts: i32,
}

#[derive(Default)]
pub struct HighScores {
	pub ticks: Vec<i32>,
	pub steps: Vec<i32>,
	pub attempts: Vec<i32>,
}

#[derive(Default)]
pub struct SaveData {
	pub current_level: i32,
	pub unlocked_levels: Vec<bool>,
	pub completed_levels: Vec<bool>,
	pub high_scores: HighScores,
	pub ephemeral: bool,
	pub show_hidden_levels: bool,
	pub segmented_speedrun: bool,
	pub options: chipty::OptionsDto,
}

impl SaveData {
	/// Unlocks the level and sets it as the current level.
	///
	/// When the previous level was completed or if the correct level password was entered.
	pub fn unlock_level(&mut self, level_number: i32) {
		let level_index = (level_number - 1) as usize;
		if let Some(lock) = self.unlocked_levels.get_mut(level_index) {
			*lock = true;
			self.current_level = level_number;
		}
	}

	/// Completes the level, unlocks the next level and saves the scores.
	pub fn complete_level(&mut self, level_number: i32, scores: Option<Scores>) {
		let level_index = (level_number - 1) as usize;
		if let Some(lock) = self.completed_levels.get_mut(level_index) {
			*lock = true;
		}

		// Allow skipping up to one level
		self.unlock_level(level_number + 2);
		self.unlock_level(level_number + 1);

		// Save high scores if enabled
		if let Some(scores) = scores {
			let level_index = (level_number - 1) as usize;
			if let Some(ticks_entry) = self.high_scores.ticks.get_mut(level_index) {
				if *ticks_entry < 0 || *ticks_entry > scores.ticks {
					*ticks_entry = scores.ticks;
				}
			}
			if let Some(steps_entry) = self.high_scores.steps.get_mut(level_index) {
				if *steps_entry < 0 || *steps_entry > scores.steps {
					*steps_entry = scores.steps;
				}
			}
		}
	}

	/// Increases and returns the current attempt for this level.
	pub fn increase_level_attempts(&mut self) -> i32 {
		let level_index = (self.current_level - 1) as usize;
		let Some(attempts_entry) = self.high_scores.attempts.get_mut(level_index) else { return 0 };
		*attempts_entry += 1;
		*attempts_entry
	}

	pub fn get_level_progress(&self, level_number: i32) -> Option<chipty::LevelProgress> {
		let level_index = (level_number - 1) as usize;
		if self.completed_levels.get(level_index).copied().unwrap_or(false) {
			return Some(chipty::LevelProgress::Completed);
		}
		if self.unlocked_levels.get(level_index).copied().unwrap_or(false) {
			return Some(chipty::LevelProgress::Unlocked);
		}
		if self.show_hidden_levels {
			return Some(chipty::LevelProgress::Locked);
		}
		return None;
	}

	pub fn get_time_high_score(&self, level_number: i32) -> i32 {
		let level_index = (level_number - 1) as usize;
		self.high_scores.ticks.get(level_index).copied().unwrap_or(-1)
	}
	pub fn get_steps_high_score(&self, level_number: i32) -> i32 {
		let level_index = (level_number - 1) as usize;
		self.high_scores.steps.get(level_index).copied().unwrap_or(-1)
	}
	pub fn get_attempts(&self, level_number: i32) -> i32 {
		let level_index = (level_number - 1) as usize;
		self.high_scores.attempts.get(level_index).copied().unwrap_or(0)
	}

	pub fn save(&self, level_set: &LevelSet) {
		if self.ephemeral {
			return;
		}

		let file_name = get_levelset_state_filename(level_set);

		let level_name = level_set.levels.get((self.current_level - 1) as usize).map(|level| level.name.clone());
		let unlocked_levels = (0..level_set.levels.len()).filter_map(|level_index| {
			if !self.unlocked_levels[level_index] {
				return None;
			}
			Some(level_set.levels[level_index].name.clone())
		});
		let completed_levels = (0..level_set.levels.len()).filter_map(|level_index| {
			if !self.completed_levels[level_index] {
				return None;
			}
			Some(level_set.levels[level_index].name.clone())
		});

		let save_data = SaveFileDto {
			current_level: level_name,
			unlocked_levels: unlocked_levels.collect(),
			completed_levels: completed_levels.collect(),
			high_scores: HighScoresDto {
				ticks: self.high_scores.ticks.iter().enumerate().filter(|(_, &v)| v >= 0).map(|(i, &v)| (level_set.levels[i].name.clone(), v)).collect(),
				steps: self.high_scores.steps.iter().enumerate().filter(|(_, &v)| v >= 0).map(|(i, &v)| (level_set.levels[i].name.clone(), v)).collect(),
				attempts: self.high_scores.attempts.iter().enumerate().filter(|(_, &v)| v > 0).map(|(i, &v)| (level_set.levels[i].name.clone(), v)).collect(),
			},
			options: self.options.clone(),
		};

		let content = serde_json::to_string_pretty(&save_data).unwrap();
		match write_file(&path::Path::new(&file_name), &content) {
			Ok(_) => {}
			Err(e) => eprintln!("Error saving {}: {}", file_name, e),
		}
	}

	pub fn load(&mut self, level_set: &LevelSet) {
		*self = load(level_set, self.ephemeral);
	}
}

fn fresh(level_set: &LevelSet, ephemeral: bool) -> SaveData {
	let mut this = SaveData {
		current_level: 0,
		unlocked_levels: vec![false; level_set.levels.len()],
		completed_levels: vec![false; level_set.levels.len()],
		high_scores: HighScores {
			ticks: vec![-1; level_set.levels.len()],
			steps: vec![-1; level_set.levels.len()],
			attempts: vec![0; level_set.levels.len()],
		},
		ephemeral,
		show_hidden_levels: level_set.unlock_all_levels,
		segmented_speedrun: false,
		options: chipty::OptionsDto::default(),
	};
	if let Some(first) = this.unlocked_levels.first_mut() {
		*first = true;
	}
	this
}

fn get_levelset_state_filename(level_set: &LevelSet) -> String {
	format!("save/{}/state.json", level_set.name)
}

fn load(level_set: &LevelSet, ephemeral: bool) -> SaveData {
	if ephemeral {
		return fresh(level_set, true);
	}

	let file_name = get_levelset_state_filename(level_set);

	let Ok(content) = read_file(path::Path::new(&file_name)) else {
		return fresh(level_set, false);
	};
	let Some(dto) = serde_json::from_str::<SaveFileDto>(&content).ok() else {
		return fresh(level_set, false);
	};

	from_save_file(level_set, &dto)
}

fn from_save_file(level_set: &LevelSet, dto: &SaveFileDto) -> SaveData {
	let mut this = fresh(level_set, false);

	if let Some(current_level) = &dto.current_level {
		if let Some(level_number) = level_set.get_level_number(current_level) {
			this.current_level = level_number;
		}
	}

	this.options = dto.options.clone();

	for level_index in dto.unlocked_levels.iter().filter_map(|level_name| level_set.get_level_index(level_name)) {
		this.unlocked_levels[level_index] = true;
	}

	for level_index in dto.completed_levels.iter().filter_map(|level_name| level_set.get_level_index(level_name)) {
		this.completed_levels[level_index] = true;
	}

	fn load_high_scores(
		level_set: &LevelSet,
		saved_data: &BTreeMap<String, i32>,
		scores: &mut Vec<i32>,
	) {
		for (level_name, score) in saved_data {
			if let Some(level_index) = level_set.get_level_index(level_name) {
				scores[level_index] = *score;
			}
		}
	}
	load_high_scores(level_set, &dto.high_scores.ticks, &mut this.high_scores.ticks);
	load_high_scores(level_set, &dto.high_scores.steps, &mut this.high_scores.steps);
	load_high_scores(level_set, &dto.high_scores.attempts, &mut this.high_scores.attempts);

	return this;
}
