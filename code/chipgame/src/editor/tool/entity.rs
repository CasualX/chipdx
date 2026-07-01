use super::*;

#[derive(Clone, Default)]
pub struct EntityToolState {
	pub selected_ent: Option<chipcore::EditEntityId>,
	pub selected_args: Option<EntityArgs>,
}

impl fmt::Display for EntityToolState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let Some(args) = &self.selected_args {
			write!(f, "{:?}", args.kind)
		}
		else {
			f.write_str("None")
		}
	}
}

impl EntityToolState {
	pub fn left_click(&mut self, s: &mut EditorEditState, pressed: bool) {
		let cursor_pos = s.cursor_pos;
		if pressed {
			// Sample from the existing entities
			let ehandle = s.fx.edit.entity_at(cursor_pos);
			if let Some(ehandle) = ehandle {
				self.selected_ent = Some(ehandle);
				self.selected_args = None;
				if let Some(ent) = s.fx.edit.entity(ehandle) {
					self.selected_args = Some(*ent);
					println!("Selected: {:?} at {}", ent.kind, ent.pos);
				}
			}
			// Otherwise create a new entity
			else {
				if let Some(args) = self.selected_args {
					self.selected_ent = Some(s.fx.create_entity(EntityArgs { kind: args.kind, pos: cursor_pos, face_dir: args.face_dir }));
				}
			}
		}
		else {
			// If we have a selected entity and the cursor has moved, move the entity
			if let Some(selected_ent) = self.selected_ent {
				if let Some(ent) = s.fx.edit.entity(selected_ent) {
					if ent.pos != cursor_pos {
						s.fx.move_entity(selected_ent, cursor_pos);
					}
				}
			}
		}
	}

	pub fn cancel_left_click(&mut self, _s: &mut EditorEditState) {
		self.selected_ent = None;
	}

	pub fn think(&mut self, _s: &mut EditorEditState) {
	}

	pub fn right_click(&mut self, s: &mut EditorEditState, pressed: bool) {
		let cursor_pos = s.cursor_pos;
		if pressed {
			// Sample from the existing entities
			let ehandle = s.fx.edit.entity_at(cursor_pos);
			if let Some(ehandle) = ehandle {
				// First select the entity
				self.selected_ent = Some(ehandle);
				self.selected_args = None;
				if let Some(ent) = s.fx.edit.entity(ehandle) {
					self.selected_args = Some(*ent);
					// Then rotate the entity
					let kind = ent.kind;
					let pos = ent.pos;
					if s.fx.rotate_entity(ehandle) {
						if let Some(args) = s.fx.edit.entity(ehandle) {
							self.selected_args = Some(*args);
						}
						println!("Rotated: {:?} at {}", kind, pos);
					}
				}
			}
		}
	}

	pub fn delete(&mut self, s: &mut EditorEditState, pressed: bool) {
		if pressed {
			if self.selected_ent.is_none() {
				let cursor_pos = s.cursor_pos;
				self.selected_ent = s.fx.edit.entity_at(cursor_pos);
			}
			if let Some(selected_ent) = self.selected_ent {
				if let Some(ent) = s.fx.edit.entity(selected_ent) {
					let kind = ent.kind;
					let pos = ent.pos;
					s.fx.remove_entity(selected_ent);
					println!("Deleted: {:?} at {}", kind, pos);
				}
			}
			self.selected_ent = None;
		}
	}
}
