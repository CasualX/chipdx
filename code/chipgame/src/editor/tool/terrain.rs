use super::*;

#[derive(Clone)]
pub struct TerrainToolState {
	pub selected_terrain: chipty::Terrain,
}

impl Default for TerrainToolState {
	fn default() -> TerrainToolState {
		TerrainToolState {
			selected_terrain: chipty::Terrain::Floor,
		}
	}
}

impl fmt::Display for TerrainToolState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", self.selected_terrain)
	}
}

impl TerrainToolState {
	pub fn left_click(&mut self, s: &mut EditorEditState, pressed: bool) {
		if pressed {
			self.put(s);
		}
	}

	pub fn right_click(&mut self, s: &mut EditorEditState, pressed: bool) {
		if pressed {
			s.sample();
		}
	}

	pub fn think(&mut self, s: &mut EditorEditState) {
		if s.input.left_click {
			self.put(s);
		}
	}

	fn put(&mut self, s: &mut EditorEditState) {
		if s.input.key_shift {
			flood_fill(s, s.cursor_pos, self.selected_terrain);
		}
		else {
			s.fx.set_terrain(s.cursor_pos, self.selected_terrain);
		}
	}
}

static OFFSETS: [Vec2i; 4] = [
	Vec2i(1, 0),
	Vec2i(-1, 0),
	Vec2i(0, 1),
	Vec2i(0, -1),
];

fn flood_fill(s: &mut EditorEditState, start: Vec2i, terrain: chipty::Terrain) {
	let width = s.fx.edit.width;
	let height = s.fx.edit.height;
	if start.x < 0 || start.y < 0 || start.x >= width || start.y >= height {
		return;
	}

	let original = s.fx.edit.get_terrain(start);
	if original == terrain {
		return;
	}

	let mut stack = Vec::new();
	s.fx.set_terrain(start, terrain);
	stack.push(start);

	while let Some(pos) = stack.pop() {
		for &offset in &OFFSETS {
			let neighbor = pos + offset;
			if neighbor.x < 0 || neighbor.y < 0 || neighbor.x >= width || neighbor.y >= height {
				continue;
			}
			if s.fx.edit.get_terrain(neighbor) != original {
				continue;
			}
			s.fx.set_terrain(neighbor, terrain);
			stack.push(neighbor);
		}
	}
}
