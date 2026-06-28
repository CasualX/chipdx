use super::*;

/// Time state.
#[derive(Copy, Clone, Default)]
pub enum TimeState {
	/// Wait for player input to start the game.
	#[default]
	Waiting,
	/// Game is running.
	Running,
	/// Game is paused.
	Paused,
	/// Game over.
	GameOver,
}

/// Game state.
#[derive(Clone, Default)]
pub struct GameState {
	pub time: i32,
	pub time_state: TimeState,
	pub ps: PlayerState,
	pub field: Field,
	pub ents: EntityMap,
	pub qt: QuadTree,
	pub templates: HashMap<Vec2i, EntityArgs>,
	pub rand: Random,
	pub events: Events,
	pub input: Input,
	pub inputs: Vec<u8>,
}

impl GameState {
	pub fn parse(&mut self, ld: &chipty::LevelDto, rng_seed: RngSeed) {
		let seed = match rng_seed {
			RngSeed::Manual(seed) => seed,
			RngSeed::System => {
				let mut seed = [0u8; 8];
				urandom::rng::getentropy(&mut seed);
				u64::from_le_bytes(seed)
			},
		};

		self.time = 0;
		self.time_state = TimeState::Running;
		self.ps = PlayerState::default();
		self.field.parse(ld, seed);
		self.ents.clear();
		self.qt.init(ld.map.width, ld.map.height);
		self.templates.clear();
		self.rand.reseed(seed);
		self.events.clear();
		self.input = Input::default();
		self.inputs.clear();

		// Create entities
		for data in &ld.entities {
			if matches!(self.field.get_terrain(data.pos), Terrain::CloneMachine) {
				self.templates.entry(data.pos).or_insert(*data);
			}
			else {
				self.entity_create(data);
			}
		}
		// And update their hidden flags
		for ehandle in self.ents.handles() {
			if let Some(mut ent) = self.ents.take(ehandle) {
				if matches!(self.field.get_terrain(ent.pos), Terrain::BearTrap) {
					bear_trap(self, &mut ent);
				}
				if matches!(ent.kind, EntityKind::Block | EntityKind::IceBlock) {
					update_hidden_fire(self, ent.pos, true);
				}
				else {
					update_hidden_flag(self, &mut ent);
				}
				self.ents.put(ent);
			}
		}
		// Initialize player state
		ps_init_player(self);
	}
}

impl GameState {
	/// Advance the game state by one tick.
	pub fn tick(&mut self, input: &Input) {
		// Remove entities marked for removal at the start of the tick
		// This allows observers to see entities for the tick they are removed
		for ehandle in self.ents.handles() {
			fn check_remove(ent: &Entity) -> bool {
				ent.flags & EF_REMOVE != 0
			}
			if self.ents.get(ehandle).map(check_remove).unwrap_or(false) {
				self.entity_remove(ehandle);
			}
		}

		// Wait for the player to press any direction input to start the game
		match self.time_state {
			TimeState::Paused => return,
			TimeState::Waiting => if !input.has_directional_input() { return },
			TimeState::Running => {},
			TimeState::GameOver => return,
		}
		self.time_state = TimeState::Running;

		// Check if the player has run out of time
		if self.field.time_limit > 0 && self.time >= self.field.time_limit * FPS {
			ps_game_over(self, GameOverReason::TimeOut);
			return;
		}

		// Handle player input
		ps_input(self, input);

		// Movement phase
		{
			let mut phase = MovementPhase::default();
			for ehandle in self.ents.handles() {
				if let Some(mut ent) = self.ents.take(ehandle) {
					(ent.data.movement_phase)(self, &mut phase, &mut ent);
					self.ents.put(ent);
				}
			}
		}

		// Action phase
		{
			let mut phase = ActionPhase::default();
			for ehandle in self.ents.handles() {
				if let Some(mut ent) = self.ents.take(ehandle) {
					(ent.data.action_phase)(self, &mut phase, &mut ent);
					self.ents.put(ent);
				}
			}
		}

		// Terrain phase
		let mut phase = TerrainPhase::default();
		for ehandle in self.ents.handles() {
			if let Some(mut ent) = self.ents.take(ehandle) {
				(ent.data.terrain_phase)(self, &mut phase, &mut ent);
				self.ents.put(ent);
			}
		}
		if phase.toggle_walls & 1 != 0 {
			self.toggle_walls();
		}
		if phase.turn_around_tanks & 1 != 0 {
			self.turn_around_tanks();
		}

		self.input = *input;
		self.time += 1;

		// HACK: Spawn the cloned entities on the 'next' tick
		// Otherwise the clones won't behave correctly...
		// * Collides with the entity triggering the button one tile away
		// * Does not trigger buttons as they are spawned
		self.spawn_clones(&phase.spawns);
	}

	/// Returns the trap state at the given position.
	pub fn get_trap_state(&self, pos: Vec2i) -> TrapState {
		let mut state = TrapState::Active;
		for conn in &self.field.conns {
			if conn.dest == pos {
				for ehandle in self.qt.get(conn.src) {
					if self.ents.is_valid(ehandle) {
						state = TrapState::Inactive;
					}
				}
			}
		}
		return state;
	}

	/// Spawn cloned entities from spawners.
	pub fn spawn_clones(&mut self, spawns: &[EntityArgs]) {
		let mut phase = &mut MovementPhase::default();

		for args in spawns {
			// Clones are forced out of the spawner, so they must have a direction
			let Some(face_dir) = args.face_dir else { continue };

			let ehandle = self.entity_create(args);
			if let Some(mut ent) = self.ents.take(ehandle) {
				// Force the new entity to move out of the spawner
				let success = try_move(self, &mut phase, &mut ent, face_dir);
				// Speed up the entity if spawned onto ice or force floors
				let terrain = self.field.get_terrain(ent.pos);
				if matches!(terrain, Terrain::Ice | Terrain::IceNE | Terrain::IceSE | Terrain::IceNW | Terrain::IceSW | Terrain::ForceW | Terrain::ForceE | Terrain::ForceN | Terrain::ForceS | Terrain::ForceRandom) {
					ent.step_spd = ent.base_spd / 2;
				}
				self.ents.put(ent);
				// If the entity movement out of the spawner fails, remove it
				// This indicates that there's a lot of entities being spawned
				if !success {
					self.entity_remove(ehandle);
				}
			}
		}
	}

	/// Sets terrain at position and fires event if changed.
	pub fn set_terrain(&mut self, pos: Vec2i, terrain: Terrain) {
		if let Some(old) = self.field.set_terrain(pos, terrain) {
			self.events.fire(GameEvent::TerrainUpdated { pos, old, new: terrain });
			// TODO: Update hidden flags when terrain changes to fire?
		}
	}

	/// Returns true if the player is standing on a hint tile.
	pub fn is_show_hint(&self) -> bool {
		for &ehandle in &self.ps.ents {
			if let Some(ent) = self.ents.get(ehandle) {
				let terrain = self.field.get_terrain(ent.pos);
				if matches!(terrain, Terrain::Hint) {
					return true;
				}
			}
		}
		return false;
	}

	/// Toggle all toggleable walls and floors on the field.
	pub fn toggle_walls(&mut self) {
		for y in 0..self.field.height {
			for x in 0..self.field.width {
				let terrain = self.field.get_terrain(Vec2i::new(x, y));
				let new = match terrain {
					Terrain::ToggleFloor => Terrain::ToggleWall,
					Terrain::ToggleWall => Terrain::ToggleFloor,
					_ => continue,
				};
				self.set_terrain(Vec2i::new(x, y), new);
			}
		}
	}

	/// Turn around all tanks on the field.
	pub fn turn_around_tanks(&mut self) {
		for other in self.ents.iter_mut() {
			if matches!(other.kind, EntityKind::Tank) {
				if let Some(face_dir) = other.face_dir {
					other.face_dir = Some(face_dir.turn_around());
					self.events.fire(GameEvent::EntityTurn { entity: other.handle });
				}
			}
		}
	}

	/// Trigger game over state.
	pub fn game_over(&mut self, reason: GameOverReason) {
		// Play game over jingle
		let sound = match reason {
			GameOverReason::LevelComplete => Some(SoundFx::GameWin),
			GameOverReason::Drowned => Some(SoundFx::WaterSplash),
			GameOverReason::Burned => Some(SoundFx::FireWalking),
			GameOverReason::Bombed => None, // Already fired by the Bomb entity!
			GameOverReason::Collided => Some(SoundFx::GameOver),
			GameOverReason::Eaten => Some(SoundFx::GameOver),
			GameOverReason::TimeOut => Some(SoundFx::GameOver),
			GameOverReason::NotOkay => None,
		};
		if let Some(sound) = sound {
			self.events.fire(GameEvent::SoundFx { sound });
		}

		self.events.fire(GameEvent::GameOver { reason });
		self.time_state = TimeState::GameOver;
	}

	/// Returns true if the game is over.
	pub fn is_game_over(&self) -> bool {
		matches!(self.time_state, TimeState::GameOver)
	}

	/// Returns true if the game should advance one tick in step mode.
	pub fn should_tick_step_mode(&self, input: &Input) -> bool {
		if input.b || input.has_directional_input() {
			return true;
		}
		let Some(player) = self.ents.get(self.ps.master) else {
			return true;
		};
		// When the player is in the middle of a step
		if self.time < player.step_time + player.step_spd {
			return true;
		}
		// When the player is trapped
		if player.is_trapped() {
			return true;
		}
		// Check terrain conditions
		let terrain = self.field.get_terrain(player.pos);
		if matches!(terrain, Terrain::Exit | Terrain::Teleport) {
			return true;
		}
		if !self.ps.boots.has(Boots::ICE_SKATES) && matches!(terrain, Terrain::Ice | Terrain::IceNE | Terrain::IceSE | Terrain::IceNW | Terrain::IceSW) {
			return true;
		}
		if !self.ps.boots.has(Boots::FLIPPERS) && matches!(terrain, Terrain::Water) {
			return true;
		}
		if !self.ps.boots.has(Boots::FIRE_BOOTS) && matches!(terrain, Terrain::Fire) {
			return true;
		}
		return false;
	}

	/// Reset player input state.
	pub fn input_reset(&mut self) {
		self.input = Input::default();
		self.ps.inbuf = InputBuffer::default();
	}

	/// Save replay data.
	pub fn save_replay(&self, realtime: f32) -> chipty::ReplayDto {
		chipty::ReplayDto {
			level_name: Some(self.field.name.clone()),
			date: None,
			attempts: self.ps.attempts,
			realtime,
			ticks: self.time,
			steps: self.ps.steps,
			bonks: self.ps.bonks,
			seed: format!("{:016x}", self.field.seed),
			inputs: chipty::encode(&self.inputs),
			unpauses: 0,
			warps_set: 0,
			warps_used: 0,
		}
	}
}
