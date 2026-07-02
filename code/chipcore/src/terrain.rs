use super::*;

#[derive(Default)]
pub struct TerrainPhase {
	pub toggle_walls: u32,
	pub turn_around_tanks: u32,
	pub spawns: Vec<EntityArgs>,
}

pub fn bear_trap(s: &mut GameState, ent: &mut Entity) {
	if matches!(s.get_trap_state(ent.pos), TrapState::Active) {
		// Notify only if the entity is newly trapped
		if ent.flags & (EF_TRAPPED | EF_RELEASED) == 0 {
			s.events.fire(GameEvent::EntityTrapped { entity: ent.handle });
			// Avoid audio spam when the level is initially loaded
			if s.time != 0 || ent.flags & EF_NEW_POS != 0 {
				s.events.fire(GameEvent::SoundFx { sound: SoundFx::TrapEntered });
			}
		}
		ent.flags |= EF_TRAPPED;
	}
	else {
		ent.flags &= !EF_TRAPPED;
	}
}

fn is_button_press_audible(ent: &Entity) -> bool {
	matches!(ent.kind, EntityKind::Player | EntityKind::Block | EntityKind::IceBlock)
}

pub fn green_button(s: &mut GameState, phase: &mut TerrainPhase, ent: &mut Entity) {
	phase.toggle_walls += 1;
	if is_button_press_audible(ent) {
		s.events.fire(GameEvent::SoundFx { sound: SoundFx::ButtonPressed });
	}
}

pub fn red_button(s: &mut GameState, phase: &mut TerrainPhase, ent: &mut Entity) {
	for conn in &s.field.conns {
		if conn.src != ent.pos {
			continue;
		}

		// Handle CloneBlock tiles separately
		let clone_block_dir = match s.field.get_terrain(conn.dest) {
			Terrain::CloneBlockN => Some(Compass::Up),
			Terrain::CloneBlockW => Some(Compass::Left),
			Terrain::CloneBlockS => Some(Compass::Down),
			Terrain::CloneBlockE => Some(Compass::Right),
			_ => None,
		};

		// Spawn a new entity
		let args = if let Some(clone_block_dir) = clone_block_dir {
			EntityArgs {
				kind: EntityKind::Block,
				pos: conn.dest,
				face_dir: Some(clone_block_dir),
			}
		}
		else {
			// Find the template entity connected to the red button
			let Some(&template) = s.templates.get(&conn.dest) else { return };
			template
		};
		phase.spawns.push(args);

		if is_button_press_audible(ent) {
			s.events.fire(GameEvent::SoundFx { sound: SoundFx::ButtonPressed });
		}
	}
}

pub fn brown_button(s: &mut GameState, _phase: &mut TerrainPhase, ent: &mut Entity) {
	if is_button_press_audible(ent) {
		s.events.fire(GameEvent::SoundFx { sound: SoundFx::ButtonPressed });
	}
	for conn in &s.field.conns {
		if conn.src != ent.pos {
			continue;
		}
		// Release trapped entities at the destination
		for ehandle in s.qt.get(conn.dest) {
			let Some(mut ent) = s.ents.take(ehandle) else { continue };
			ent.flags |= EF_RELEASED;
			s.ents.put(ent);
		}
	}
}

pub fn blue_button(s: &mut GameState, phase: &mut TerrainPhase, ent: &mut Entity) {
	phase.turn_around_tanks += 1;
	if is_button_press_audible(ent) {
		s.events.fire(GameEvent::SoundFx { sound: SoundFx::ButtonPressed });
	}
}
