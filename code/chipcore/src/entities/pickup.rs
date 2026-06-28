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
	if ent.flags & EF_REMOVE != 0 {
		return;
	}

	update_hidden_flag(s, ent);

	if ps_check_new_pos(s, ent.pos) {
		pickup_item(s, ent);
	}
}

fn pickup_item(s: &mut GameState, ent: &mut Entity) {
	ent.flags |= EF_REMOVE;

	let (item, sound) = match ent.kind {
		EntityKind::Chip => {
			s.ps.chips += 1;
			(ItemPickup::Chip, SoundFx::ICCollected)
		},
		EntityKind::BlueKey => {
			s.ps.keys[KeyColor::Blue as usize] += 1;
			(ItemPickup::BlueKey, SoundFx::KeyCollected)
		},
		EntityKind::RedKey => {
			s.ps.keys[KeyColor::Red as usize] += 1;
			(ItemPickup::RedKey, SoundFx::KeyCollected)
		},
		EntityKind::GreenKey => {
			s.ps.keys[KeyColor::Green as usize] += 1;
			(ItemPickup::GreenKey, SoundFx::KeyCollected)
		},
		EntityKind::YellowKey => {
			s.ps.keys[KeyColor::Yellow as usize] += 1;
			(ItemPickup::YellowKey, SoundFx::KeyCollected)
		},
		EntityKind::Flippers => {
			s.ps.boots.give(Boots::FLIPPERS);
			(ItemPickup::Flippers, SoundFx::BootCollected)
		},
		EntityKind::FireBoots => {
			s.ps.boots.give(Boots::FIRE_BOOTS);
			(ItemPickup::FireBoots, SoundFx::BootCollected)
		},
		EntityKind::IceSkates => {
			s.ps.boots.give(Boots::ICE_SKATES);
			(ItemPickup::IceSkates, SoundFx::BootCollected)
		},
		EntityKind::SuctionBoots => {
			s.ps.boots.give(Boots::SUCTION_BOOTS);
			(ItemPickup::SuctionBoots, SoundFx::BootCollected)
		},
		_ => return,
	};

	s.events.fire(GameEvent::ItemPickup { entity: ent.handle, item });
	s.events.fire(GameEvent::SoundFx { sound });
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
		keys: true,
		boots: true,
		chips: true,
		creatures: true,
		player: false,
		thief: true,
		hint: false,
	},
	hidden: HiddenData {
		dirt: true,
	},
};
