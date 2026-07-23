use std::collections::{HashMap, VecDeque};

use chipty::*;

pub fn convert(dat: &super::Data, title: String) -> chipty::LevelSetDto {
	let mut levels = Vec::new();
	for lvdat in dat.levels.iter() {
		// Parse the tile layers
		let (map, entities) = parse_content(&lvdat.top_layer, &lvdat.bottom_layer);
		let entities = monster_order(&lvdat.metadata.monsters, entities);

		// Build connections
		let mut conns = Vec::new();
		if let Some(traps) = &lvdat.metadata.traps {
			for lnk in traps {
				conns.push(FieldConn {
					src: cvmath::Vec2i(lnk.brown_button_x as i32, lnk.brown_button_y as i32),
					dest: cvmath::Vec2i(lnk.trap_x as i32, lnk.trap_y as i32),
				});
			}
		}
		if let Some(cloners) = &lvdat.metadata.cloners {
			for lnk in cloners {
				conns.push(FieldConn {
					src: cvmath::Vec2i(lnk.red_button_x as i32, lnk.red_button_y as i32),
					dest: cvmath::Vec2i(lnk.cloner_x as i32, lnk.cloner_y as i32),
				});
			}
		}

		let mut level = LevelDto {
			name: lvdat.metadata.title.clone().unwrap(),
			author: lvdat.metadata.author.clone(),
			hint: lvdat.metadata.hint.clone(),
			password: lvdat.metadata.password.clone(),
			time_limit: lvdat.time_limit as i32,
			required_chips: lvdat.required_chips as i32,
			map,
			entities,
			connections: conns,
			camera_triggers: Vec::new(),
			replays: None,
			trophies: None,
		};
		level.normalize();

		replace_block_cloners(&mut level);

		levels.push(LevelRef::Direct(level));
	}

	chipty::LevelSetDto {
		name: title.clone(),
		title,
		subtitle: None,
		difficulty: 0,
		about: None,
		splash: None,
		levels,
	}
}

fn process_tile(terrain: &mut Vec<Terrain>, entities: &mut Vec<EntityArgs>, pos: cvmath::Vec2<i32>, tile: u8) {
	let index = (pos.y * 32 + pos.x) as usize;
	match tile {
		0x00 => (),//terrain[index] = Terrain::Floor,
		0x01 => terrain[index] = Terrain::Wall,
		0x02 => entities.push(ent_args(EntityKind::Chip, pos, None)),
		0x03 => terrain[index] = Terrain::Water,
		0x04 => terrain[index] = Terrain::Fire,
		0x05 => terrain[index] = Terrain::InvisibleWall,
		0x06 => terrain[index] = Terrain::ThinWallN,
		0x07 => terrain[index] = Terrain::ThinWallW,
		0x08 => terrain[index] = Terrain::ThinWallS,
		0x09 => terrain[index] = Terrain::ThinWallE,
		0x0a => entities.push(ent_args(EntityKind::Block, pos, None)),
		0x0b => terrain[index] = Terrain::Dirt,
		0x0c => terrain[index] = Terrain::Ice,
		0x0d => terrain[index] = Terrain::ForceS,
		0x0e => terrain[index] = Terrain::CloneBlockN,
		0x0f => terrain[index] = Terrain::CloneBlockW,

		0x10 => terrain[index] = Terrain::CloneBlockS,
		0x11 => terrain[index] = Terrain::CloneBlockE,
		0x12 => terrain[index] = Terrain::ForceN,
		0x13 => terrain[index] = Terrain::ForceE,
		0x14 => terrain[index] = Terrain::ForceW,
		0x15 => terrain[index] = Terrain::Exit,
		0x16 => terrain[index] = Terrain::BlueLock,
		0x17 => terrain[index] = Terrain::RedLock,
		0x18 => terrain[index] = Terrain::GreenLock,
		0x19 => terrain[index] = Terrain::YellowLock,
		0x1a => terrain[index] = Terrain::IceNW,
		0x1b => terrain[index] = Terrain::IceNE,
		0x1c => terrain[index] = Terrain::IceSE,
		0x1d => terrain[index] = Terrain::IceSW,
		0x1e => terrain[index] = Terrain::FakeBlueWall,
		0x1f => terrain[index] = Terrain::RealBlueWall,

		0x20 => terrain[index] = Terrain::Blank, // Used internally as the Combination tile.
		0x21 => entities.push(ent_args(EntityKind::Thief, pos, None)),
		0x22 => entities.push(ent_args(EntityKind::Socket, pos, None)),
		0x23 => terrain[index] = Terrain::GreenButton,
		0x24 => terrain[index] = Terrain::RedButton,
		0x25 => terrain[index] = Terrain::ToggleWall,
		0x26 => terrain[index] = Terrain::ToggleFloor,
		0x27 => terrain[index] = Terrain::BrownButton,
		0x28 => terrain[index] = Terrain::BlueButton,
		0x29 => terrain[index] = Terrain::Teleport,
		0x2a => entities.push(ent_args(EntityKind::Bomb, pos, None)),
		0x2b => terrain[index] = Terrain::BearTrap,
		0x2c => terrain[index] = Terrain::HiddenWall,
		0x2d => terrain[index] = Terrain::Gravel,
		0x2e => terrain[index] = Terrain::RecessedWall,
		0x2f => terrain[index] = Terrain::Hint,

		0x30 => terrain[index] = Terrain::ThinWallSE,
		0x31 => terrain[index] = Terrain::CloneMachine,
		0x32 => terrain[index] = Terrain::ForceRandom,
		0x33 => terrain[index] = Terrain::WaterHazard, // Drowned Chip
		0x34 => { terrain[index] = Terrain::Fire; entities.push(ent_args(EntityKind::PlayerNPC, pos, None)) }, // Burned Chip
		0x35 => { terrain[index] = Terrain::Floor; entities.push(ent_args(EntityKind::PlayerNPC, pos, None)) }, // Burned Chip
		0x36 => terrain[index] = Terrain::InvisibleWall, // This byte does not correspond to any defined tile, but acts like an invisible wall.
		0x37 => terrain[index] = Terrain::InvisibleWall, // This byte does not correspond to any defined tile, but acts like an invisible wall.
		0x38 => entities.push(ent_args(EntityKind::IceBlock, pos, None)),
		0x39 => terrain[index] = Terrain::FakeExit,
		0x3a => terrain[index] = Terrain::FakeExit,
		0x3b => terrain[index] = Terrain::FakeExit,
		0x3c => { terrain[index] = Terrain::Water; entities.push(ent_args(EntityKind::Player, pos, Some(Compass::Up))) }, // Swimming Chip (N)
		0x3d => { terrain[index] = Terrain::Water; entities.push(ent_args(EntityKind::Player, pos, Some(Compass::Left))) }, // Swimming Chip (W)
		0x3e => { terrain[index] = Terrain::Water; entities.push(ent_args(EntityKind::Player, pos, Some(Compass::Down))) }, // Swimming Chip (S)
		0x3f => { terrain[index] = Terrain::Water; entities.push(ent_args(EntityKind::Player, pos, Some(Compass::Right))) }, // Swimming Chip (E)

		0x40 => entities.push(ent_args(EntityKind::Bug, pos, Some(Compass::Up))),
		0x41 => entities.push(ent_args(EntityKind::Bug, pos, Some(Compass::Left))),
		0x42 => entities.push(ent_args(EntityKind::Bug, pos, Some(Compass::Down))),
		0x43 => entities.push(ent_args(EntityKind::Bug, pos, Some(Compass::Right))),

		0x44 => entities.push(ent_args(EntityKind::FireBall, pos, Some(Compass::Up))),
		0x45 => entities.push(ent_args(EntityKind::FireBall, pos, Some(Compass::Left))),
		0x46 => entities.push(ent_args(EntityKind::FireBall, pos, Some(Compass::Down))),
		0x47 => entities.push(ent_args(EntityKind::FireBall, pos, Some(Compass::Right))),

		0x48 => entities.push(ent_args(EntityKind::PinkBall, pos, Some(Compass::Up))),
		0x49 => entities.push(ent_args(EntityKind::PinkBall, pos, Some(Compass::Left))),
		0x4a => entities.push(ent_args(EntityKind::PinkBall, pos, Some(Compass::Down))),
		0x4b => entities.push(ent_args(EntityKind::PinkBall, pos, Some(Compass::Right))),

		0x4c => entities.push(ent_args(EntityKind::Tank, pos, Some(Compass::Up))),
		0x4d => entities.push(ent_args(EntityKind::Tank, pos, Some(Compass::Left))),
		0x4e => entities.push(ent_args(EntityKind::Tank, pos, Some(Compass::Down))),
		0x4f => entities.push(ent_args(EntityKind::Tank, pos, Some(Compass::Right))),

		0x50 => entities.push(ent_args(EntityKind::Glider, pos, Some(Compass::Up))),
		0x51 => entities.push(ent_args(EntityKind::Glider, pos, Some(Compass::Left))),
		0x52 => entities.push(ent_args(EntityKind::Glider, pos, Some(Compass::Down))),
		0x53 => entities.push(ent_args(EntityKind::Glider, pos, Some(Compass::Right))),

		0x54 => entities.push(ent_args(EntityKind::Teeth, pos, Some(Compass::Up))),
		0x55 => entities.push(ent_args(EntityKind::Teeth, pos, Some(Compass::Left))),
		0x56 => entities.push(ent_args(EntityKind::Teeth, pos, Some(Compass::Down))),
		0x57 => entities.push(ent_args(EntityKind::Teeth, pos, Some(Compass::Right))),

		0x58 => entities.push(ent_args(EntityKind::Walker, pos, Some(Compass::Up))),
		0x59 => entities.push(ent_args(EntityKind::Walker, pos, Some(Compass::Left))),
		0x5a => entities.push(ent_args(EntityKind::Walker, pos, Some(Compass::Down))),
		0x5b => entities.push(ent_args(EntityKind::Walker, pos, Some(Compass::Right))),

		0x5c => entities.push(ent_args(EntityKind::Blob, pos, Some(Compass::Up))),
		0x5d => entities.push(ent_args(EntityKind::Blob, pos, Some(Compass::Left))),
		0x5e => entities.push(ent_args(EntityKind::Blob, pos, Some(Compass::Down))),
		0x5f => entities.push(ent_args(EntityKind::Blob, pos, Some(Compass::Right))),

		0x60 => entities.push(ent_args(EntityKind::Paramecium, pos, Some(Compass::Up))),
		0x61 => entities.push(ent_args(EntityKind::Paramecium, pos, Some(Compass::Left))),
		0x62 => entities.push(ent_args(EntityKind::Paramecium, pos, Some(Compass::Down))),
		0x63 => entities.push(ent_args(EntityKind::Paramecium, pos, Some(Compass::Right))),

		0x64 => entities.push(ent_args(EntityKind::BlueKey, pos, None)),
		0x65 => entities.push(ent_args(EntityKind::RedKey, pos, None)),
		0x66 => entities.push(ent_args(EntityKind::GreenKey, pos, None)),
		0x67 => entities.push(ent_args(EntityKind::YellowKey, pos, None)),
		0x68 => entities.push(ent_args(EntityKind::Flippers, pos, None)),
		0x69 => entities.push(ent_args(EntityKind::FireBoots, pos, None)),
		0x6a => entities.push(ent_args(EntityKind::IceSkates, pos, None)),
		0x6b => entities.push(ent_args(EntityKind::SuctionBoots, pos, None)),
		0x6c => entities.push(ent_args(EntityKind::Player, pos, None)),
		0x6d => entities.push(ent_args(EntityKind::Player, pos, None)),
		0x6e => entities.push(ent_args(EntityKind::Player, pos, None)),
		0x6f => entities.push(ent_args(EntityKind::Player, pos, None)),

		0x70 => terrain[index] = Terrain::Blank, // Values of $70 and above are used internally for graphical transparency.
		value => unimplemented!("Tile: ${:02x}", value),
	}
}

fn parse_content(upper: &[u8], lower: &[u8]) -> (FieldDto, Vec<EntityArgs>) {
	let mut terrain = vec![Terrain::Floor; 32 * 32];
	let mut entities = Vec::new();

	for y in 0..32 {
		for x in 0..32 {
			let index = y * 32 + x;
			let pos = cvmath::Vec2i(x as i32, y as i32);

			process_tile(&mut terrain, &mut entities, pos, lower[index]);
			process_tile(&mut terrain, &mut entities, pos, upper[index]);
		}
	}

	// Replace Block entites on Floor with DirtBlock terrain
	entities.retain_mut(|ent| {
		let index = (ent.pos.y * 32 + ent.pos.x) as usize;
		if matches!(ent.kind, EntityKind::Block) && matches!(terrain[index], Terrain::Floor | Terrain::DirtBlock) {
			terrain[index] = Terrain::DirtBlock;
			return false;
		}
		true
	});

	let mut legend = Vec::new();
	let data = {
		let mut legend_map = HashMap::new();
		legend_map.insert(Terrain::Blank, 0); legend.push(Terrain::Blank);
		legend_map.insert(Terrain::Floor, 1); legend.push(Terrain::Floor);
		let mut idx = 2;
		for &terrain in terrain.iter() {
			if !legend_map.contains_key(&terrain) {
				legend_map.insert(terrain, idx);
				legend.push(terrain);
				idx += 1;
			}
		}
		terrain.iter().map(|&terrain| legend_map[&terrain]).collect()
	};

	let map = FieldDto { width: 32, height: 32, data, legend };
	return (map, entities);
}

fn ent_args(kind: EntityKind, pos: cvmath::Vec2i, face_dir: Option<Compass>) -> EntityArgs {
	EntityArgs { kind, pos, face_dir }
}

fn replace_block_cloners(level: &mut LevelDto) -> bool {
	let mut fixed = false;

	let mut ents_to_remove = Vec::new();

	// Replace Block entities targetted by a red connection with CloneBlock terrain
	for (ent_index, ent_args) in level.entities.iter().enumerate() {
		if !matches!(ent_args.kind, EntityKind::Block) {
			continue;
		}

		let Some(&conn) = level.connections.iter().find(|&conn| conn.dest == ent_args.pos) else {
			continue
		};

		{
			let index = (conn.src.y * level.map.width + conn.src.x) as usize;
			let tile = level.map.data[index] as usize;
			let terrain = level.map.legend[tile];
			if !matches!(terrain, Terrain::RedButton) {
				continue;
			}
		}

		let new_terrain = match ent_args.face_dir {
			Some(Compass::Up) => Terrain::CloneBlockN,
			Some(Compass::Down) => Terrain::CloneBlockS,
			Some(Compass::Left) => Terrain::CloneBlockW,
			Some(Compass::Right) => Terrain::CloneBlockE,
			_ => continue,
		};

		ents_to_remove.push(ent_index);

		let new_tile = {
			if let Some(new_tile) = level.map.legend.iter().position(|&t| t == new_terrain) {
				new_tile as u8
			}
			else {
				level.map.legend.push(new_terrain);
				level.map.legend.len() as u8 - 1
			}
		};

		let index = (ent_args.pos.y * level.map.width + ent_args.pos.x) as usize;
		level.map.data[index] = new_tile;
		fixed = true;
	}

	for &ent_index in ents_to_remove.iter().rev() {
		level.entities.remove(ent_index);
	}

	return fixed;
}

fn monster_order(monsters: &Option<Vec<crate::Monster>>, entities: Vec<EntityArgs>) -> Vec<EntityArgs> {
	let Some(monsters) = monsters else { return entities };
	if monsters.is_empty() {
		return entities;
	}

	let original_entities_len = entities.len();

	let mut others = Vec::with_capacity(entities.len());
	let mut monster_map: HashMap<(i32, i32), VecDeque<EntityArgs>> = HashMap::new();
	let mut monster_order = Vec::new();

	// First, separate monsters from other entities.
	for ent in entities {
		let is_monster = matches!(ent.kind,
			| EntityKind::Bug
			| EntityKind::FireBall
			| EntityKind::PinkBall
			| EntityKind::Tank
			| EntityKind::Glider
			| EntityKind::Teeth
			| EntityKind::Walker
			| EntityKind::Blob
			| EntityKind::Paramecium
		);
		if is_monster {
			let key = (ent.pos.x, ent.pos.y);
			monster_map
				.entry(key)
				.or_insert_with(VecDeque::new)
				.push_back(ent);
			monster_order.push(key);
		}
		else {
			others.push(ent);
		}
	}

	if monster_map.is_empty() {
		return others;
	}

	let mut ordered = Vec::new();

	// Next, order monsters according to the specified order in metadata.
	for monster in monsters {
		let key = (monster.x as i32, monster.y as i32);
		let mut remove_bucket = false;
		if let Some(queue) = monster_map.get_mut(&key) {
			if let Some(ent) = queue.pop_front() {
				ordered.push(ent);
			}
			if queue.is_empty() {
				remove_bucket = true;
			}
		}
		if remove_bucket {
			monster_map.remove(&key);
		}
	}

	// Finally, append any remaining monsters in the order they were found.
	for key in monster_order {
		let mut remove_bucket = false;
		if let Some(queue) = monster_map.get_mut(&key) {
			if let Some(ent) = queue.pop_front() {
				ordered.push(ent);
			}
			if queue.is_empty() {
				remove_bucket = true;
			}
		}
		if remove_bucket {
			monster_map.remove(&key);
		}
	}

	others.extend(ordered);

	assert_eq!(others.len(), original_entities_len);
	return others;
}
