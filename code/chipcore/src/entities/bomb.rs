use super::*;

pub fn create(s: &mut GameState, args: &EntityArgs) -> EntityHandle {
	let handle = s.ents.alloc();
	s.ents.put(Entity {
		data: &DATA,
		handle,
		kind: args.kind,
		pos: args.pos,
		base_spd: BASE_SPD,
		face_dir: args.face_dir,
		step_dir: None,
		step_spd: BASE_SPD,
		step_time: 0,
		flags: 0,
	});
	s.qt.add(handle, args.pos);
	return handle;
}

fn movement_phase(s: &mut GameState, phase: &mut MovementPhase, ent: &mut Entity) {
	if s.time >= ent.step_time + ent.step_spd {
		try_terrain_move(s, phase, ent);
	}
}

fn action_phase(s: &mut GameState, _phase: &mut ActionPhase, ent: &mut Entity) {
	update_hidden_flag(s, ent);

	let mut exploded = false;
	for ehandle in s.qt.get(ent.pos) {
		if ehandle == ent.handle {
			continue;
		}
		let Some(other_ent) = s.ents.get_mut(ehandle) else { continue };
		// Only explode when an entity moved into the bomb
		if !(ent.flags & EF_NEW_POS != 0 || other_ent.flags & EF_NEW_POS != 0) {
			continue;
		}
		exploded = true;
		// Remove non-player entities
		if matches!(other_ent.kind, EntityKind::Player) {
			ps_attack(s, ehandle, GameOverReason::Bombed);
		}
		else {
			other_ent.flags |= EF_REMOVE;
		}
	}

	if exploded {
		ent.flags |= EF_REMOVE;
		s.events.fire(GameEvent::BombExplode { pos: ent.pos });
		s.events.fire(GameEvent::SoundFx { sound: SoundFx::BombExplosion });
	}
}

fn terrain_phase(s: &mut GameState, phase: &mut TerrainPhase, ent: &mut Entity) {
	let terrain = s.field.get_terrain(ent.pos);

	if matches!(terrain, Terrain::BearTrap) {
		return bear_trap(s, ent);
	}

	if ent.has_moved(s.time) {
		match terrain {
			Terrain::Water => {
				s.events.fire(GameEvent::SoundFx { sound: SoundFx::WaterSplash });
				s.events.fire(GameEvent::WaterSplash { pos: ent.pos });
				ent.flags |= EF_REMOVE;
			}
			Terrain::Fire => {
				s.events.fire(GameEvent::BombExplode { pos: ent.pos });
				s.events.fire(GameEvent::SoundFx { sound: SoundFx::BombExplosion });
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
		gravel: false,
		fire: false,
		dirt: true,
		water: false,
		exit: true,
		blue_fake: true,
		recessed_wall: true,
		keys: false,
		boots: false,
		chips: false,
		creatures: false,
		player: false,
		thief: false,
		hint: false,
	},
	hidden: HiddenData {
		dirt: false,
	},
};
