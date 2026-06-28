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
		step_dir: args.face_dir,
		step_spd: BASE_SPD,
		step_time: -BASE_SPD,
		// Start with momentum if facing a direction
		// CCLP2: Level 28: Madness I
		flags: if args.face_dir.is_some() { EF_MOMENTUM } else { 0 },
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
	// Allow Player to start under a Block
	if ent.flags & EF_NEW_POS != 0 {
		ps_attack_pos(s, ent.pos, GameOverReason::Collided);
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
				s.set_terrain(ent.pos, Terrain::Dirt);
				s.events.fire(GameEvent::SoundFx { sound: SoundFx::WaterSplash });
				s.events.fire(GameEvent::WaterSplash { pos: ent.pos });
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
		boots: true,
		chips: true,
		creatures: true,
		player: false,
		thief: true,
		hint: false,
	},
	hidden: HiddenData {
		dirt: false,
	},
};
