use std::collections::HashMap;

use super::*;

/// Stable entity id used by the core editor model.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct EditEntityId(usize);

/// Core state for editing level data without coupling to gameplay simulation.
#[derive(Clone, Default)]
pub struct EditState {
	pub name: String,
	pub author: Option<String>,
	pub hint: Option<String>,
	pub password: Option<String>,
	pub time_limit: i32,
	pub required_chips: i32,
	pub width: i32,
	pub height: i32,
	pub terrain: Vec<Terrain>,
	pub next_entity_id: usize,
	pub entities: HashMap<EditEntityId, EntityArgs>,
	pub entity_order: Vec<EditEntityId>,
	pub connections: Vec<FieldConn>,
	pub camera_triggers: Vec<CameraFocusTrigger>,
	pub replays: Option<Vec<ReplayDto>>,
	pub trophies: Option<Trophies>,
}

impl EditState {
	pub fn from_level_dto(level: &LevelDto) -> EditState {
		assert!(
			level.map.width >= chipty::FIELD_MIN_WIDTH && level.map.width <= FIELD_MAX_WIDTH &&
			level.map.height >= chipty::FIELD_MIN_HEIGHT && level.map.height <= FIELD_MAX_HEIGHT,
			"Invalid map size width={} height={}", level.map.width, level.map.height);

		let size = level.map.width as usize * level.map.height as usize;
		let mut terrain = Vec::with_capacity(size);
		if level.map.data.is_empty() {
			terrain.resize(size, Terrain::Floor);
		}
		else {
			assert_eq!(level.map.data.len(), size, "Invalid map data length");
			for &tile in &level.map.data {
				let terrain_index = tile as usize;
				let tile = *level.map.legend.get(terrain_index).unwrap_or_else(|| panic!("Invalid terrain legend index {}", terrain_index));
				terrain.push(tile);
			}
		}

		let mut state = EditState {
			name: level.name.clone(),
			author: level.author.clone(),
			hint: level.hint.clone(),
			password: level.password.clone(),
			time_limit: level.time_limit,
			required_chips: level.required_chips,
			width: level.map.width,
			height: level.map.height,
			terrain,
			connections: level.connections.clone(),
			camera_triggers: level.camera_triggers.clone(),
			replays: level.replays.clone(),
			trophies: level.trophies.clone(),
			..EditState::default()
		};

		for &args in &level.entities {
			state.create_entity(args);
		}

		state
	}

	pub fn to_level_dto(&self) -> LevelDto {
		assert_eq!(self.terrain.len(), (self.width * self.height) as usize, "Invalid edit terrain data length");

		let mut legend = vec![Terrain::Blank, Terrain::Floor];
		let mut data = Vec::with_capacity(self.terrain.len());
		for &terrain in &self.terrain {
			let index = match legend.iter().position(|&tile| tile == terrain) {
				Some(index) => index,
				None => {
					legend.push(terrain);
					legend.len() - 1
				},
			};
			data.push(index as u8);
		}

		let entities: Vec<_> = self.entity_order.iter()
			.filter_map(|id| self.entities.get(id))
			.copied()
			.collect();

		let mut level = LevelDto {
			name: self.name.clone(),
			author: self.author.clone(),
			hint: self.hint.clone(),
			password: self.password.clone(),
			required_chips: self.required_chips,
			time_limit: self.time_limit,
			map: FieldDto {
				width: self.width,
				height: self.height,
				data,
				legend,
			},
			entities: entities.clone(),
			connections: self.connections.clone(),
			camera_triggers: self.camera_triggers.clone(),
			replays: self.replays.clone(),
			trophies: self.trophies.clone(),
		};
		level.normalize();
		level.entities = entities;
		level
	}

	pub fn is_pos_inside(&self, pos: Vec2i) -> bool {
		pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.height
	}

	pub fn index(&self, pos: Vec2i) -> Option<usize> {
		if self.is_pos_inside(pos) {
			Some((pos.y * self.width + pos.x) as usize)
		}
		else {
			None
		}
	}

	pub fn get_terrain(&self, pos: Vec2i) -> Terrain {
		self.index(pos)
			.and_then(|index| self.terrain.get(index))
			.copied()
			.unwrap_or(Terrain::Wall)
	}

	pub fn set_terrain(&mut self, pos: Vec2i, terrain: Terrain) -> Option<Terrain> {
		let ptr = self.index(pos).and_then(|index| self.terrain.get_mut(index))?;
		let old = *ptr;
		if old == terrain {
			return None;
		}
		*ptr = terrain;
		Some(old)
	}

	pub fn resize(&mut self, left: i32, top: i32, right: i32, bottom: i32, fill_terrain: Terrain) -> bool {
		let new_width = self.width + left + right;
		let new_height = self.height + top + bottom;
		if new_width < FIELD_MIN_WIDTH || new_width > FIELD_MAX_WIDTH ||
			new_height < FIELD_MIN_HEIGHT || new_height > FIELD_MAX_HEIGHT {
			return false;
		}

		let old_width = self.width;
		let old_height = self.height;
		let old_terrain = self.terrain.clone();
		let mut new_terrain = vec![fill_terrain; (new_width * new_height) as usize];
		for old_y in 0..old_height {
			for old_x in 0..old_width {
				let new_x = old_x + left;
				let new_y = old_y + top;
				if new_x >= 0 && new_x < new_width && new_y >= 0 && new_y < new_height {
					let old_index = (old_y * old_width + old_x) as usize;
					let new_index = (new_y * new_width + new_x) as usize;
					new_terrain[new_index] = old_terrain[old_index];
				}
			}
		}

		self.width = new_width;
		self.height = new_height;
		self.terrain = new_terrain;

		let offset = Vec2i::new(left, top);
		let entity_ids = self.entity_order.clone();
		for id in entity_ids {
			if let Some(args) = self.entities.get_mut(&id) {
				args.pos += offset;
				if !Self::is_pos_inside_dims(args.pos, new_width, new_height) {
					self.remove_entity(id);
				}
			}
		}

		for conn in &mut self.connections {
			conn.src += offset;
			conn.dest += offset;
		}
		self.connections.retain(|conn|
			Self::is_pos_inside_dims(conn.src, new_width, new_height) &&
			Self::is_pos_inside_dims(conn.dest, new_width, new_height));

		for trigger in &mut self.camera_triggers {
			trigger.player_pos += offset;
		}
		self.camera_triggers.retain(|trigger| Self::is_pos_inside_dims(trigger.player_pos, new_width, new_height));

		true
	}

	pub fn entity_at(&self, pos: Vec2i) -> Option<EditEntityId> {
		self.entity_order.iter()
			.copied()
			.find(|id| self.entities.get(id).map(|args| args.pos == pos).unwrap_or(false))
	}

	pub fn entity(&self, id: EditEntityId) -> Option<&EntityArgs> {
		self.entities.get(&id)
	}

	pub fn entity_mut(&mut self, id: EditEntityId) -> Option<&mut EntityArgs> {
		self.entities.get_mut(&id)
	}

	pub fn entities_in_order(&self) -> impl Iterator<Item = (EditEntityId, &EntityArgs)> {
		self.entity_order.iter()
			.copied()
			.filter_map(|id| self.entities.get(&id).map(|args| (id, args)))
	}

	pub fn create_entity(&mut self, args: EntityArgs) -> EditEntityId {
		let id = EditEntityId(self.next_entity_id);
		self.next_entity_id += 1;
		self.entities.insert(id, args);
		self.entity_order.push(id);
		id
	}

	pub fn remove_entity(&mut self, id: EditEntityId) -> Option<EntityArgs> {
		let args = self.entities.remove(&id)?;
		self.entity_order.retain(|&other| other != id);
		Some(args)
	}

	pub fn move_entity(&mut self, id: EditEntityId, pos: Vec2i) -> bool {
		let Some(args) = self.entities.get_mut(&id) else { return false };
		args.pos = pos;
		true
	}

	pub fn rotate_entity(&mut self, id: EditEntityId) -> bool {
		let Some(args) = self.entities.get_mut(&id) else { return false };
		args.face_dir = next_face_dir(args.face_dir);
		true
	}

	pub fn swap_entity_order(&mut self, a: EditEntityId, b: EditEntityId) -> bool {
		let Some(a_index) = self.entity_order.iter().position(|&id| id == a) else { return false };
		let Some(b_index) = self.entity_order.iter().position(|&id| id == b) else { return false };
		self.entity_order.swap(a_index, b_index);
		true
	}

	pub fn brush_create(&self) -> LevelBrush {
		LevelBrush {
			width: self.width,
			height: self.height,
			terrain: self.terrain.iter().copied().map(Some).collect(),
			entities: self.entities_in_order().map(|(_, args)| *args).collect(),
			connections: self.connections.clone(),
		}
	}

	pub fn brush_apply(&mut self, pos: Vec2i, brush: &LevelBrush) {
		assert!(brush.width > 0 && brush.height > 0, "Invalid brush size");
		assert_eq!(brush.terrain.len(), (brush.width * brush.height) as usize, "Invalid brush terrain data length");

		for by in 0..brush.height {
			for bx in 0..brush.width {
				if let Some(terrain) = brush.terrain[(by * brush.width + bx) as usize] {
					self.set_terrain(pos + Vec2i::new(bx, by), terrain);
				}
			}
		}

		for &ent_args in &brush.entities {
			self.create_entity(EntityArgs {
				pos: pos + ent_args.pos,
				..ent_args
			});
		}

		for conn in &brush.connections {
			let src = if brush.is_pos_inside(conn.src) { pos + conn.src } else { conn.src };
			let dest = if brush.is_pos_inside(conn.dest) { pos + conn.dest } else { conn.dest };
			if self.is_pos_inside(src) && self.is_pos_inside(dest) {
				self.connections.push(FieldConn { src, dest });
			}
		}
	}

	pub fn toggle_connection(&mut self, conn: FieldConn) {
		if let Some(index) = self.connections.iter().position(|&other| other == conn) {
			self.connections.remove(index);
		}
		else {
			self.connections.push(conn);
		}
	}

	fn is_pos_inside_dims(pos: Vec2i, width: i32, height: i32) -> bool {
		pos.x >= 0 && pos.x < width && pos.y >= 0 && pos.y < height
	}
}

fn next_face_dir(face_dir: Option<Compass>) -> Option<Compass> {
	match face_dir {
		Some(Compass::Up) => Some(Compass::Right),
		Some(Compass::Right) => Some(Compass::Down),
		Some(Compass::Down) => Some(Compass::Left),
		Some(Compass::Left) => None,
		None => Some(Compass::Up),
	}
}
