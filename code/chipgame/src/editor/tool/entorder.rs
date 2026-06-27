use super::*;

#[derive(Clone, Default)]
pub struct EntOrderToolState {
}

impl EntOrderToolState {
	pub fn left_click(&mut self, s: &mut EditorEditState, pressed: bool) {
		if pressed {
			self.move_entity(s, true);
		}
	}

	pub fn right_click(&mut self, s: &mut EditorEditState, pressed: bool) {
		if pressed {
			self.move_entity(s, false);
		}
	}

	pub fn think(&mut self, _s: &mut EditorEditState) {
	}

	fn move_entity(&self, s: &mut EditorEditState, inc: bool) {
		let cursor_pos = s.cursor_pos;
		let entities: Vec<_> = s.fx.edit.entities_in_order().map(|(id, args)| (id, *args)).collect();
		if let Some(index) = entities.iter().position(|(_, ent)| ent.pos == cursor_pos) {
			let new_index = if inc { usize::min(index + 1, entities.len() - 1) } else { index.saturating_sub(1) };
			if index != new_index {
				s.fx.swap_entity_order(entities[index].0, entities[new_index].0);
			}
		}
	}
}
