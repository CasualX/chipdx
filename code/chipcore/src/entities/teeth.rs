use super::*;

pub fn create(s: &mut GameState, args: &EntityArgs) -> EntityHandle {
	let handle = s.ents.alloc();
	s.ents.put(Entity {
		data: &DATA,
		handle,
		kind: args.kind,
		pos: args.pos,
		base_spd: BASE_SPD * 2,
		face_dir: args.face_dir,
		step_dir: None,
		step_spd: BASE_SPD * 2,
		step_time: 0,
		flags: EF_MOMENTUM, // HACK: Spawn teeth with momentum, fixes Walls of CC1: Level 148
	});
	s.qt.add(handle, args.pos);
	return handle;
}

fn movement_phase(s: &mut GameState, phase: &mut MovementPhase, ent: &mut Entity) {
	if s.time >= ent.step_time + ent.step_spd {
		if try_terrain_move(s, phase, ent) { }
		else if let Some((first_dir, second_dir)) = chase_dirs(s, ent) {
			if try_move(s, phase, ent, first_dir) { }
			else if try_move(s, phase, ent, second_dir) { }
			// If no legal move, stay put and face in the first direction
			else {
				if ent.face_dir != Some(first_dir) {
					s.events.fire(GameEvent::EntityTurn { entity: ent.handle });
				}
				ent.face_dir = Some(first_dir);
			}
		}
	}
}

fn chase_dirs(s: &GameState, ent: &Entity) -> Option<(Compass, Compass)> {
	let pl = ps_nearest_ent(s, ent.pos)?;
	let Vec2i { x, y } = pl.pos - ent.pos;

	// Teeth moves either vertically or horizontally toward Chip one square at a time, always taking the longer path, and vertically if tied.
	// However, if this move would be illegal because of some obstacle, it will go the other way if that is a legal move, and if not,
	// it will stay put until Chip moves somewhere that allows it to make another move.

	if x == 0 && y == 0 {
		None
	}
	else if x.abs() <= -y {
		if x > 0 {
			Some((Compass::Up, Compass::Right))
		}
		else if x < 0 {
			Some((Compass::Up, Compass::Left))
		}
		else {
			Some((Compass::Up, Compass::Up))
		}
	}
	else if x.abs() <= y {
		if x > 0 {
			Some((Compass::Down, Compass::Right))
		}
		else if x < 0 {
			Some((Compass::Down, Compass::Left))
		}
		else {
			Some((Compass::Down, Compass::Down))
		}
	}
	else if x < 0 {
		if y > 0 {
			Some((Compass::Left, Compass::Down))
		}
		else if y < 0 {
			Some((Compass::Left, Compass::Up))
		}
		else {
			Some((Compass::Left, Compass::Left))
		}
	}
	else if x > 0 {
		if y > 0 {
			Some((Compass::Right, Compass::Down))
		}
		else if y < 0 {
			Some((Compass::Right, Compass::Up))
		}
		else {
			Some((Compass::Right, Compass::Right))
		}
	}
	else {
		None
	}
}

fn action_phase(s: &mut GameState, _phase: &mut ActionPhase, ent: &mut Entity) {
	update_hidden_flag(s, ent);

	ps_attack_pos(s, ent.pos, GameOverReason::Eaten);
}

fn terrain_phase(s: &mut GameState, phase: &mut TerrainPhase, ent: &mut Entity) {
	let terrain = s.field.get_terrain(ent.pos);

	if matches!(terrain, Terrain::BearTrap) {
		return bear_trap(s, ent);
	}

	if ent.has_moved(s.time) {
		match terrain {
			Terrain::Water => {
				s.events.fire(GameEvent::WaterSplash { pos: ent.pos });
				ent.flags |= EF_REMOVE;
			}
			Terrain::Fire => {
				ent.flags |= EF_REMOVE;
			}
			Terrain::GreenButton => green_button(s, phase, ent),
			Terrain::RedButton => red_button(s, phase, ent),
			Terrain::BrownButton => brown_button(s, phase, ent),
			Terrain::BlueButton => blue_button(s, phase, ent),
			_ => {}
		}
	}
}

static DATA: EntityData = EntityData {
	movement_phase,
	action_phase,
	terrain_phase,
	flags: SolidFlags {
		gravel: true,
		fire: true,
		dirt: true,
		water: false,
		exit: true,
		blue_fake: true,
		recessed_wall: true,
		keys: false,
		boots: true,
		chips: true,
		creatures: true,
		player: false,
		thief: true,
		hint: true,
	},
	hidden: HiddenData {
		dirt: false,
	},
};
