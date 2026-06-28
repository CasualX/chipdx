use super::*;

/// Time after which Chip returns to idle animation
const IDLE_TIME: i32 = 20;

pub fn create(s: &mut GameState, args: &EntityArgs) -> EntityHandle {
	let handle = s.ents.alloc();
	s.ents.put(Entity {
		data: &DATA,
		handle,
		kind: args.kind,
		pos: args.pos,
		base_spd: BASE_SPD,
		face_dir: None,
		step_dir: None,
		step_spd: BASE_SPD,
		step_time: -BASE_SPD,
		flags: 0,
	});
	s.qt.add(handle, args.pos);
	return handle;
}

fn movement_phase(s: &mut GameState, phase: &mut MovementPhase, ent: &mut Entity) {
	// Freeze player if game over
	if s.is_game_over() {
		return;
	}

	let terrain = s.field.get_terrain(ent.pos);

	// Clear movement after a delay
	if s.time >= ent.step_time + IDLE_TIME {
		if ent.face_dir.is_some() {
			s.events.fire(GameEvent::EntityTurn { entity: ent.handle });
		}
		ent.face_dir = None;
	}

	// Wait until movement is cleared before accepting new input
	if s.time >= ent.step_time + ent.step_spd {
		let input_dir = s.ps.inbuf.read_dir();

		if s.ps.dev_wtw {
			if let Some(input_dir) = input_dir {
				try_move(s, phase, ent, input_dir);
				return;
			}
		}

		'end_move: {
			if let Some(step_dir) = ent.step_dir {
				if matches!(terrain, Terrain::Teleport) {
					if teleport(s, phase, ent, step_dir) {
						break 'end_move;
					}
				}

				// Handle ice physics
				if !s.ps.boots.has(Boots::ICE_SKATES) && matches!(terrain, Terrain::Ice | Terrain::IceNW | Terrain::IceNE | Terrain::IceSW | Terrain::IceSE) {
					let (ice_dir, back_dir) = match step_dir {
						Compass::Up => match terrain {
							Terrain::IceNW => (Compass::Right, Compass::Down),
							Terrain::IceNE => (Compass::Left, Compass::Down),
							Terrain::IceSE => (Compass::Up, Compass::Left),
							Terrain::IceSW => (Compass::Up, Compass::Right),
							_ => (step_dir, step_dir.turn_around()),
						},
						Compass::Left => match terrain {
							Terrain::IceNW => (Compass::Down, Compass::Right),
							Terrain::IceNE => (Compass::Left, Compass::Down),
							Terrain::IceSE => (Compass::Left, Compass::Up),
							Terrain::IceSW => (Compass::Up, Compass::Right),
							_ => (step_dir, step_dir.turn_around()),
						},
						Compass::Down => match terrain {
							Terrain::IceNW => (Compass::Down, Compass::Right),
							Terrain::IceNE => (Compass::Down, Compass::Left),
							Terrain::IceSE => (Compass::Left, Compass::Up),
							Terrain::IceSW => (Compass::Right, Compass::Up),
							_ => (step_dir, step_dir.turn_around()),
						},
						Compass::Right => match terrain {
							Terrain::IceNW => (Compass::Right, Compass::Down),
							Terrain::IceNE => (Compass::Down, Compass::Left),
							Terrain::IceSE => (Compass::Up, Compass::Left),
							Terrain::IceSW => (Compass::Right, Compass::Up),
							_ => (step_dir, step_dir.turn_around()),
						},
					};
					// If the player is blocked, try to turn around
					if !try_move(s, phase, ent, ice_dir) {
						if !try_move(s, phase, ent, back_dir) {
							// Softlocked!
						}
						else {
							s.events.fire(GameEvent::PlayerBump { entity: ent.handle });
							s.events.fire(GameEvent::SoundFx { sound: SoundFx::CantMove });
						}
					}
					// Ice is considered a forced move, so players can change direction on the next force floor
					// Clearing this flag is handled below, in the force floor handling code
					ent.flags |= EF_TERRAIN_MOVE;
					break 'end_move;
				}
			}

			if ent.is_trapped() {
				break 'end_move;
			}

			// Handle force terrain
			let force_dir = match terrain {
				_ if s.ps.boots.has(Boots::SUCTION_BOOTS) => None,
				Terrain::ForceW => Some(Compass::Left),
				Terrain::ForceE => Some(Compass::Right),
				Terrain::ForceN => Some(Compass::Up),
				Terrain::ForceS => Some(Compass::Down),
				Terrain::ForceRandom => Some(s.rand.next()),
				_ => None,
			};
			if let Some(force_dir) = force_dir {
				let override_dir =
					// Allow the player to override the force terrain every other step
					if ent.flags & EF_TERRAIN_MOVE == 0 { None }
					// Allow any direction to override random forces, to make them less frustrating
					else if matches!(terrain, Terrain::ForceRandom) { input_dir }
					// Only allow perpendicular directions to override cardinal forces
					else {
						match force_dir {
							Compass::Left | Compass::Right => match input_dir {
								Some(Compass::Up) => Some(Compass::Up),
								Some(Compass::Down) => Some(Compass::Down),
								_ => None,
							},
							Compass::Up | Compass::Down => match input_dir {
								Some(Compass::Left) => Some(Compass::Left),
								Some(Compass::Right) => Some(Compass::Right),
								_ => None,
							},
						}
					};

				// Consider this a forced move if the player did not step in the direction of the force terrain
				if let Some(override_dir) = override_dir {
					if try_move(s, phase, ent, override_dir) {
						ent.flags &= !EF_TERRAIN_MOVE;
					}
					else {
						try_move(s, phase, ent, force_dir);
						bump(s, ent, override_dir);
						ent.flags |= EF_TERRAIN_MOVE;
					}
				}
				else {
					try_move(s, phase, ent, force_dir);
					ent.flags |= EF_TERRAIN_MOVE;
				}

				break 'end_move;
			}
			else {
				ent.flags &= !EF_TERRAIN_MOVE;
			}

			// Handle player input
			if let Some(step_dir) = input_dir {
				if !try_move(s, phase, ent, step_dir) {
					bump(s, ent, step_dir);
				}
			}
		}
	}
}

fn bump(s: &mut GameState, ent: &mut Entity, dir: Compass) {
	// ent.step_spd = ent.base_spd;
	ent.step_time = s.time;
	ent.face_dir = Some(dir);
	s.ps.bonks += 1;
	s.events.fire(GameEvent::EntityTurn { entity: ent.handle });
	s.events.fire(GameEvent::PlayerBump { entity: ent.handle });
	// Play sound only for the master player
	if s.ps.master == ent.handle {
		s.events.fire(GameEvent::SoundFx { sound: SoundFx::CantMove });
	}
}

fn action_phase(s: &mut GameState, _phase: &mut ActionPhase, ent: &mut Entity) {
	update_hidden_flag(s, ent);

	let activity = match s.field.get_terrain(ent.pos) {
		Terrain::Water => {
			PlayerActivity::Swimming
		},
		Terrain::Ice | Terrain::IceNE | Terrain::IceNW | Terrain::IceSE | Terrain::IceSW => {
			if s.ps.boots.has(Boots::ICE_SKATES) {
				PlayerActivity::IceSkating
			}
			else {
				PlayerActivity::IceSliding
			}
		}
		Terrain::ForceN | Terrain::ForceW | Terrain::ForceS | Terrain::ForceE | Terrain::ForceRandom => {
			if s.ps.boots.has(Boots::SUCTION_BOOTS) {
				PlayerActivity::ForceWalking
			}
			else {
				PlayerActivity::ForceSliding
			}
		}
		_ => PlayerActivity::Walking,
	};
	ps_activity(s, ent.handle, activity);
}

fn terrain_phase(s: &mut GameState, phase: &mut TerrainPhase, ent: &mut Entity) {
	if let Some(step_dir) = ent.step_dir {
		let from_pos = ent.pos - step_dir.to_vec();
		// HACK: Avoid triggering the recessed wall on the first step
		if matches!(s.field.get_terrain(from_pos), Terrain::RecessedWall) && s.ps.steps > 1 {
			s.set_terrain(from_pos, Terrain::Wall);
			s.events.fire(GameEvent::SoundFx { sound: SoundFx::WallPopup });
		}
	}

	let terrain = s.field.get_terrain(ent.pos);

	if matches!(terrain, Terrain::BearTrap) {
		return bear_trap(s, ent);
	}

	if s.time == ent.step_time + ent.step_spd - 1 && ent.flags & EF_NEW_POS != 0 {
		if matches!(terrain, Terrain::Fire) && !s.ps.boots.has(Boots::FIRE_BOOTS) {
			ps_attack(s, ent.handle, GameOverReason::Burned);
			return;
		}
		if matches!(terrain, Terrain::Water) && !s.ps.boots.has(Boots::FLIPPERS) {
			ps_attack(s, ent.handle, GameOverReason::Drowned);
			return;
		}
		if matches!(terrain, Terrain::Exit) {
			ps_attack(s, ent.handle, GameOverReason::LevelComplete);
			return;
		}
	}

	if ent.has_moved(s.time) {
		match terrain {
			Terrain::Dirt => {
				s.set_terrain(ent.pos, Terrain::Floor);
				s.events.fire(GameEvent::SoundFx { sound: SoundFx::TileEmptied });
			}
			Terrain::Water => water_splash(s, ent),
			Terrain::FakeBlueWall => {
				s.set_terrain(ent.pos, Terrain::Floor);
				s.events.fire(GameEvent::SoundFx { sound: SoundFx::BlueWallCleared });
			}
			Terrain::GreenButton => green_button(s, phase, ent),
			Terrain::RedButton => red_button(s, phase, ent),
			Terrain::BrownButton => brown_button(s, phase, ent),
			Terrain::BlueButton => blue_button(s, phase, ent),
			_ => {}
		}
	}
}

fn water_splash(s: &mut GameState, ent: &Entity) {
	let Some(step_dir) = ent.step_dir else { return };
	let old_pos = ent.pos - step_dir.to_vec();
	if !matches!(s.field.get_terrain(old_pos), Terrain::Water) {
		s.events.fire(GameEvent::WaterSplash { pos: ent.pos });
	}
}

static DATA: EntityData = EntityData {
	movement_phase,
	action_phase,
	terrain_phase,
	flags: SolidFlags {
		gravel: false,
		fire: false,
		dirt: false,
		water: false,
		exit: false,
		blue_fake: false,
		recessed_wall: false,
		keys: false,
		boots: false,
		chips: false,
		creatures: false,
		player: true,
		thief: false,
		hint: false,
	},
	hidden: HiddenData {
		dirt: false,
	},
};
