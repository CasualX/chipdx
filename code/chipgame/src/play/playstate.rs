use super::*;

#[derive(Default)]
pub struct PlayState {
	pub fx: Option<Box<fx::FxState>>,
	pub warp: Option<Box<fx::FxState>>,
	pub menu: menu::MenuState,
	pub events: Vec<PlayEvent>,
	pub lvsets: LevelSets,
	pub save_data: SaveData,
	pub metrics: shade::DrawMetrics,
	pub single_level_session: bool,
}

impl cvar::IVisit for PlayState {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		if let Some(fx) = &mut self.fx {
			fx.visit(f);
		}
	}
}

impl PlayState {
	pub fn load_single_level(&mut self, level_set: LevelSet) {
		self.single_level_session = true;
		self.fx = None;
		self.warp = None;
		self.menu.close_all();
		self.lvsets.selected = 0;
		self.lvsets.collection.clear();
		self.lvsets.collection.push(level_set);
		self.save_data.load(self.lvsets.current());
	}

	pub fn launch(&mut self, g: &mut dyn shade::IGraphics) {
		if self.lvsets.collection.is_empty() {
			return;
		}
		let mut splash = Vec::new();
		for level_set in &self.lvsets.collection {
			let Some(data) = &level_set.splash
			else {
				splash.push(None);
				continue
			};
			let props = shade::TextureProps::default();
			if let Ok(animated_image) = shade::image::AnimatedImage::load_memory(data) {
				let animated_texture = g.animated_image(&animated_image, &props);
				splash.push(Some(animated_texture));
			}
		}
		self.menu.stack.push(menu::Menu::LevelSet(menu::LevelSetMenu {
			selected: i32::max(0, self.lvsets.selected) as usize,
			items: self.lvsets.collection.iter().map(|level_set| level_set.title.clone()).collect(),
			external_loader_label: self.lvsets.external_loader_label.clone(),
			splash,
			ntime: 0,
		}));
	}

	pub fn think(&mut self, input: &chipcore::Input) {
		let menu_active = self.menu.think(input);

		if let Some(fx) = &mut self.fx {
			fx.step_mode = self.save_data.options.step_mode;
			fx.assist_dist = if self.save_data.options.assist_mode { 2 } else { 0 };
			fx.think(input, menu_active);
		}

		self.sync();
	}

	pub fn play_level(&mut self, level_number: i32) {
		// If loading a level fails just... do nothing
		let Some(level) = self.lvsets.current().levels.get((level_number - 1) as usize)
		else { return };

		// Update save data
		self.save_data.current_level = level_number;
		let attempts = self.save_data.increase_level_attempts();
		self.save_data.save(&self.lvsets.current());

		// Load the segmented replay if available
		let (seed, inputs);
		let load_segmented_replay = || {
			if !self.save_data.segmented_speedrun {
				return None;
			}
			let Some(fx) = &self.fx else {
				return None;
			};
			if fx.level_number != level_number {
				return None;
			}
			return Some(fx);
		};
		if let Some(fx) = load_segmented_replay() {
			seed = chipcore::RngSeed::Manual(fx.game.field.seed);
			inputs = Some(fx.game.inputs.clone());
		}
		// else if let Ok(replay) = fs::read_to_string(format!("code/chipcore/tests/replays/{}/level{level_number}.json", self.lvsets.current().name)) {
		// 	let replay: chipty::ReplayDto = serde_json::from_str(&replay).unwrap();
		// 	seed = chipcore::RngSeed::Manual(u64::from_str_radix(&replay.seed, 16).unwrap());
		// 	inputs = Some(chipty::decode(&replay.inputs));
		// }
		else {
			seed = chipcore::RngSeed::System;
			inputs = None;
		};

		// Create the FX state
		let mut fx = fx::FxState::new(level_number, level, seed, tiles::tile_gfx);
		fx.game.ps.attempts = attempts;
		fx.replay_inputs = inputs;
		fx.camera.set_perspective(self.save_data.options.perspective);
		fx.camera.set_zoom_mode(self.save_data.options.zoom_mode, false, fx.time);
		self.fx = Some(fx);
		self.warp = None;

		self.menu.close_all();
		self.events.push(PlayEvent::SetTitle);
		self.play_music();
	}

	fn preview_level(&mut self, level_number: i32) {
		self.fx = if let Some(level) = self.lvsets.current().levels.get((level_number - 1) as usize) {
			let mut fx = fx::FxState::new(level_number, level, chipcore::RngSeed::System, tiles::tile_gfx);
			fx.is_preview = true;
			fx.camera.set_perspective(self.save_data.options.perspective);
			fx.camera.set_zoom_mode(self.save_data.options.zoom_mode, false, fx.time);
			Some(fx)
		}
		else {
			None
		};
		self.warp = None;
	}

	pub fn toggle_music(&mut self) {
		// FIXME! Music can't be toggled during Select LevelSet screen
		if self.lvsets.selected < 0 {
			return;
		}
		self.save_data.options.background_music = !self.save_data.options.background_music;
		self.save_data.save(&self.lvsets.current());
		self.play_music();
	}

	fn play_music(&mut self) {
		let music = if !self.save_data.options.background_music {
			None
		}
		else if let Some(fx) = &self.fx {
			if fx.is_preview {
				Some(chipty::MusicId::MenuMusic)
			}
			else {
				Some(chipty::MusicId::GameMusic)
			}
		}
		else {
			Some(chipty::MusicId::MenuMusic)
		};
		self.events.push(PlayEvent::PlayMusic { music });
	}

	pub fn sync(&mut self) {
		let events = mem::replace(&mut self.menu.events, Vec::new());
		for evt in events {
			// eprintln!("MenuEvent: {:?}", evt);
			match evt {
				menu::MenuEvent::LoadLevelSet { index } => {
					self.events.push(PlayEvent::SetTitle);
					self.lvsets.selected = index as i32;
					self.save_data.load(&self.lvsets.current());
					self.save_data.save(&self.lvsets.current());
					self.menu.open_main(self.save_data.current_level > 0, &self.lvsets.current().title);
					self.play_music();
				}
				menu::MenuEvent::LoadExternalLevelSet => {
					self.events.push(PlayEvent::LoadExternalLevelSet);
				}
				menu::MenuEvent::NewGame => {
					self.play_level(1);
				}
				menu::MenuEvent::OpenMainMenu => {
					if self.single_level_session {
						continue;
					}
					self.fx = None;
					self.events.push(PlayEvent::SetTitle);
					self.menu.open_main(self.save_data.current_level > 0, &self.lvsets.current().title);
					self.play_music();
				}
				menu::MenuEvent::SwitchLevelSet => {
					if self.single_level_session {
						continue;
					}
					self.fx = None;
					self.lvsets.selected = -1;
					self.menu.close_all();
					self.events.push(PlayEvent::Restart);
					self.events.push(PlayEvent::SetTitle);
				}
				menu::MenuEvent::PreviewLevel { level_number } => {
					self.preview_level(level_number);
				}
				menu::MenuEvent::OpenGoToLevel => {
					// Start previewing at the current level
					if self.save_data.get_level_progress(self.save_data.current_level).is_some() {
						self.preview_level(self.save_data.current_level);
					}

					let mut menu = menu::GoToLevel::default();
					menu.load_items(&self.lvsets.current(), &self.save_data);
					self.menu.stack.push(menu::Menu::GoToLevel(menu));
				}
				menu::MenuEvent::OpenUnlockLevel => {
					let menu = menu::UnlockLevelMenu {
						selected: 0,
						password: [None; 4],
					};
					self.menu.stack.push(menu::Menu::UnlockLevel(menu));
				}
				menu::MenuEvent::EnterPassword { code } => {
					// Secret code to soft-unlock all levels
					if code == *b"CHIP" {
						self.save_data.show_hidden_levels ^= true;
						self.menu.open_main(true, &self.lvsets.current().title);
						return;
					}
					let mut success = false;
					for (index, level) in self.lvsets.current().levels.iter().enumerate() {
						if let Some(level_password) = &level.password {
							if level_password.as_bytes() == code.as_slice() {
								let level_number = index as i32 + 1;
								self.save_data.unlock_level(level_number);
								success = true;
							}
						}
					}
					if success {
						self.menu.open_main(true, &self.lvsets.current().title);
					}
				}
				menu::MenuEvent::PlayLevel { level_number } => {
					self.play_level(level_number);
				}
				menu::MenuEvent::PlayNextLevel => {
					let level_number = if let Some(fx) = &self.fx { fx.level_number + 1 } else { 1 };
					self.play_level(level_number);
				}
				menu::MenuEvent::RetryLevel | menu::MenuEvent::RestartLevel => {
					let level_number = if let Some(fx) = &self.fx { fx.level_number } else { 1 };
					self.play_level(level_number);
				}
				menu::MenuEvent::Continue => {
					let level_number = i32::max(1, self.save_data.current_level);
					self.play_level(level_number);
				}
				menu::MenuEvent::ResumePlay => {
					if let Some(fx) = &mut self.fx {
						self.menu.close_all();
						fx.resume();
					}
				}
				menu::MenuEvent::SaveReplay => {
					if let Some(fx) = &self.fx {
						save_replay(self.lvsets.current(), fx);
					}
				}
				menu::MenuEvent::OpenAbout => {
					if let Some(about) = &self.lvsets.current().about {
						let menu = menu::AboutMenu {
							text: about.clone(),
						};
						self.menu.stack.push(menu::Menu::About(menu));
					}
				}
				menu::MenuEvent::ExitGame => {
					if self.single_level_session {
						continue;
					}
					self.menu.close_all();
					self.fx = None;
					self.events.push(PlayEvent::Quit);
				}
				menu::MenuEvent::OpenOptions => {
					let menu = menu::OptionsMenu {
						selected: 0,
						options: self.save_data.options.clone(),
					};
					self.menu.stack.push(menu::Menu::Options(menu));
				}
				menu::MenuEvent::OpenPauseMenu => {
					if let Some(fx) = &mut self.fx {
						fx.pause();
					}
				}
				menu::MenuEvent::OpenScoutMode => {
					if let Some(fx) = &mut self.fx {
						fx.scout();
					}
				}
				menu::MenuEvent::SetBackgroundMusic { value } => {
					if self.save_data.options.background_music != value {
						self.save_data.options.background_music = value;
						self.save_data.save(&self.lvsets.current());
						self.play_music();
					}
				}
				menu::MenuEvent::SetSoundEffects { value } => {
					if self.save_data.options.sound_effects != value {
						self.save_data.options.sound_effects = value;
						self.save_data.save(&self.lvsets.current());
					}
				}
				menu::MenuEvent::SetDeveloperMode { value } => {
					if self.save_data.options.developer_mode != value {
						self.save_data.options.developer_mode = value;
						self.save_data.save(&self.lvsets.current());
					}
				}
				menu::MenuEvent::SetPerspective { value } => {
					if self.save_data.options.perspective != value {
						self.save_data.options.perspective = value;
						self.save_data.save(&self.lvsets.current());
						if let Some(fx) = &mut self.fx {
							fx.camera.set_perspective(value);
						}
					}
				}
				menu::MenuEvent::SetZoomMode { value } => {
					if self.save_data.options.zoom_mode != value {
						self.save_data.options.zoom_mode = value;
						self.save_data.save(&self.lvsets.current());
						if let Some(fx) = &mut self.fx {
							fx.camera.set_zoom_mode(value, true, fx.time);
						}
					}
				}
				menu::MenuEvent::SetAssistMode { value } => {
					if self.save_data.options.assist_mode != value {
						self.save_data.options.assist_mode = value;
						self.save_data.save(&self.lvsets.current());
					}
				}
				menu::MenuEvent::SetStepMode { value } => {
					if self.save_data.options.step_mode != value {
						self.save_data.options.step_mode = value;
						self.save_data.save(&self.lvsets.current());
						if let Some(fx) = &mut self.fx {
							fx.step_mode = value;
						}
					}
				}
				menu::MenuEvent::SetAutoSaveReplay { value } => {
					if self.save_data.options.auto_save_replay != value {
						self.save_data.options.auto_save_replay = value;
						self.save_data.save(&self.lvsets.current());
					}
				}
				menu::MenuEvent::SetSpeedrunMode { value } => {
					if self.save_data.options.speedrun_mode != value {
						self.save_data.options.speedrun_mode = value;
						self.save_data.save(&self.lvsets.current());
					}
				}
				menu::MenuEvent::CursorMove => {
					if self.save_data.options.sound_effects {
						self.events.push(PlayEvent::PlaySound { sound: chipty::SoundFx::CursorMove });
					}
				}
				menu::MenuEvent::CursorSelect => {
					if self.save_data.options.sound_effects {
						self.events.push(PlayEvent::PlaySound { sound: chipty::SoundFx::CursorSelect });
					}
				}
				menu::MenuEvent::CloseMenu => {
					self.menu.close_menu(self.fx.is_some());
				}
				menu::MenuEvent::PreviewExit => {
					self.fx = None;
				}
				menu::MenuEvent::SaveState => {
					if let Some(fx) = &mut self.fx {
						fx.warps_set += 1;
						self.warp = Some(fx.clone());
					}
				}
				menu::MenuEvent::LoadState => {
					if let Some(warp) = &mut self.warp {
						// Update progression state
						warp.warps_used += 1;
						warp.game.ps.attempts = self.save_data.increase_level_attempts();

						// Restore FX state
						let mut warp = warp.clone();
						warp.resume();

						// Step mode already waits for user input
						if !self.save_data.options.step_mode {
							warp.game.time_state = chipcore::TimeState::Waiting;
						}

						self.fx = Some(warp);
					}
				}
			}
		}

		if let Some(fx) = &mut self.fx {
			let events = mem::replace(&mut fx.events, Vec::new());
			for evt in events {
				// eprintln!("FxEvent: {:?}", evt);
				match evt {
					fx::FxEvent::PlaySound(sound) => play_fx_play_sound(self, sound),
					fx::FxEvent::ScoutMode => play_fx_scout(self),
					fx::FxEvent::PauseGame => play_fx_pause(self),
					fx::FxEvent::ResumePlay => play_fx_unpause(self),
					fx::FxEvent::LevelComplete => play_fx_level_complete(self),
					fx::FxEvent::GameOver => play_fx_game_over(self),
					// _ => {}
				}
			}
		}
	}

	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &fx::Resources, time: f64) {
		render::drawbg(g, resx);

		if let Some(fx) = &mut self.fx {
			fx.hud_enabled = self.menu.stack.is_empty();
			fx.draw(g, resx, time);
		}

		g.begin(&shade::BeginArgs::Immediate {
			viewport: resx.viewport,
			color: &[resx.backcolor()],
			levels: None,
			depth: Some(resx.backdepth()),
		});
		if self.fx.is_some() && !self.menu.stack.is_empty() {
			menu::darken(g, resx, 168);
		}
		self.menu.draw(g, resx, time);

		if self.save_data.options.developer_mode {
			menu::draw_metrics(g, resx, &self.metrics);
		}

		if let Some(fx) = &self.fx {
			if !fx.is_preview && self.save_data.options.speedrun_mode {
				let realtime = if fx.game_realtime > 0.0 { fx.game_realtime }
				else { (time - fx.game_start_time) as f32 };

				let step_offset = if let Some(player) = fx.game.ents.get(fx.game.ps.master) {
					player.step_time % player.base_spd
				}
				else { 0 };

				menu::PlayMetrics {
					level_number: fx.level_number,
					level_name: &fx.game.field.name,
					attempts: fx.game.ps.attempts,
					time: fx.game.time,
					realtime,
					step_offset,
					steps: fx.game.ps.steps,
					bonks: fx.game.ps.bonks,
				}.draw(g, resx);
			}
		}
		g.end();
	}
}

fn play_fx_play_sound(this: &mut PlayState, sound: chipty::SoundFx) {
	if this.save_data.options.sound_effects {
		this.events.push(PlayEvent::PlaySound { sound });
	}
}

fn play_fx_scout(this: &mut PlayState) {
	let Some(_fx) = &mut this.fx else {
		return
	};

	let menu = menu::ScoutMode::default();
	this.menu.stack.push(menu::Menu::Scout(menu));
}

fn play_fx_pause(this: &mut PlayState) {
	let Some(fx) = &this.fx else {
		return
	};

	let menu = menu::PauseMenu {
		selected: 0,
		assist_mode: this.save_data.options.assist_mode,
		has_warp: this.warp.is_some(),
		level_number: fx.level_number,
		level_name: fx.game.field.name.clone(),
	};
	this.menu.stack.push(menu::Menu::Pause(menu));
}

fn play_fx_unpause(this: &mut PlayState) {
	this.menu.close_all();
}

fn play_fx_level_complete(this: &mut PlayState) {
	let Some(fx) = &this.fx else {
		return
	};

	// Check for high scores
	let scores = savedata::Scores {
		ticks: fx.game.time,
		steps: fx.game.ps.steps,
	};
	let time_high_score = this.save_data.get_time_high_score(fx.level_number);
	let steps_high_score = this.save_data.get_steps_high_score(fx.level_number);
	let high_score =
		(time_high_score < 0 || scores.ticks < time_high_score) ||
		(steps_high_score < 0 || scores.steps < steps_high_score);
	if high_score {
		if this.save_data.options.sound_effects {
			this.events.push(PlayEvent::PlaySound { sound: chipty::SoundFx::GameWin });
		}
	}

	// Update save data
	this.save_data.complete_level(fx.level_number, Some(scores));
	this.save_data.save(&this.lvsets.current());

	// Auto-save replay if enabled or if a new high score was achieved
	if this.save_data.options.auto_save_replay || high_score {
		save_replay(this.lvsets.current(), fx);
	}

	// Show game win menu
	let menu = menu::GameWinMenu {
		selected: 0,
		level_number: fx.level_number,
		level_name: fx.game.field.name.clone(),
		attempts: fx.game.ps.attempts,
		time: fx.game.time,
		steps: fx.game.ps.steps,
		bonks: fx.game.ps.bonks,
		time_high_score,
		steps_high_score,
		..Default::default()
	};
	this.menu.stack.push(menu::Menu::GameWin(menu));
}

fn play_fx_game_over(this: &mut PlayState) {
	let Some(fx) = &this.fx else {
		return
	};

	let menu = menu::GameOverMenu {
		selected: 0,
		reason: fx.game_over,
		has_warp: this.warp.is_some(),
		level_number: fx.level_number,
		level_name: fx.game.field.name.clone(),
		attempts: fx.game.ps.attempts,
		time: fx.game.time,
		steps: fx.game.ps.steps,
		bonks: fx.game.ps.bonks,
	};
	this.menu.stack.push(menu::Menu::GameOver(menu));
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn save_replay(lvset: &LevelSet, fx: &fx::FxState) {
	let replay = fx.game.save_replay(fx.game_realtime);
	let replay = chipty::ReplayDto {
		warps_set: fx.warps_set,
		warps_used: fx.warps_used,
		unpauses: fx.unpauses,
		..replay
	};
	let record = serde_json::to_string_pretty(&replay).unwrap();
	let path = format!("save/{}/replay/level{}.attempt{}.json", lvset.name, fx.level_number, fx.game.ps.attempts);
	if let Err(err) = write_file(&path::Path::new(&path), &record) {
		eprintln!("Error saving replay: {}", err);
	}
}

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
fn save_replay(_lvset: &LevelSet, _fx: &fx::FxState) {}
