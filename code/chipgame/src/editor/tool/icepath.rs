use super::*;

#[derive(Clone, Default)]
pub struct IcePathToolState {
	pub last_pos: Option<Vec2<i32>>,
	pub last_dir: Option<chipty::Compass>,
}

impl IcePathToolState {
	pub fn left_click(&mut self, s: &mut EditorEditState, pressed: bool) {
		if pressed {
			// Start a new ice path
			s.fx.set_terrain(s.cursor_pos, chipty::Terrain::Ice);
			self.last_pos = Some(s.cursor_pos);
			self.last_dir = None;
		}
		else {
			// Stop drawing
			self.last_pos = None;
			self.last_dir = None;
		}

		s.input.left_click = pressed;
	}

	pub fn right_click(&mut self, s: &mut EditorEditState, pressed: bool) {
		if pressed {
			s.sample();
		}
	}

	pub fn think(&mut self, s: &mut EditorEditState) {
		if !s.input.left_click {
			return;
		}
		let Some(last_pos) = self.last_pos else {
			return;
		};
		let cursor_pos = s.cursor_pos;
		if cursor_pos == last_pos {
			return;
		}

		// Set the previous tile to the appropriate ice corner
		let cursor_dir = chipty::Compass::from_vec(cursor_pos - last_pos);
		if let (Some(last_dir), Some(cursor_dir)) = (self.last_dir, cursor_dir) {
			let terrain = ice_tile_for_turn(last_dir, cursor_dir);
			s.fx.set_terrain(last_pos, terrain);
		}

		// Mark the current tile as ice terrain
		s.fx.set_terrain(cursor_pos, chipty::Terrain::Ice);
		self.last_pos = Some(cursor_pos);
		self.last_dir = cursor_dir;
	}
}

fn ice_tile_for_turn(from: chipty::Compass, to: chipty::Compass) -> chipty::Terrain {
	match (from, to) {
		(chipty::Compass::Up, chipty::Compass::Right) => chipty::Terrain::IceNW,
		(chipty::Compass::Up, chipty::Compass::Left) => chipty::Terrain::IceNE,
		(chipty::Compass::Right, chipty::Compass::Down) => chipty::Terrain::IceNE,
		(chipty::Compass::Right, chipty::Compass::Up) => chipty::Terrain::IceSE,
		(chipty::Compass::Down, chipty::Compass::Left) => chipty::Terrain::IceSE,
		(chipty::Compass::Down, chipty::Compass::Right) => chipty::Terrain::IceSW,
		(chipty::Compass::Left, chipty::Compass::Down) => chipty::Terrain::IceNW,
		(chipty::Compass::Left, chipty::Compass::Up) => chipty::Terrain::IceSW,
		_ => chipty::Terrain::Ice,
	}
}
