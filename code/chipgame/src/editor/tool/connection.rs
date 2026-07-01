use super::*;

#[derive(Clone, Default)]
pub struct ConnectionToolState {
	conn_src: Vec2i,
}

impl ConnectionToolState {
	pub fn left_click(&mut self, s: &mut EditorEditState, pressed: bool) {
		if pressed {
			self.conn_src = s.cursor_pos;
		}
		else {
			let new_conn = FieldConn { src: self.conn_src, dest: s.cursor_pos };

			if new_conn.src != new_conn.dest {
				s.fx.toggle_connection(new_conn);
			}
		}
	}

	pub fn cancel_left_click(&mut self, _s: &mut EditorEditState) {
	}

	pub fn right_click(&mut self, _s: &mut EditorEditState, _pressed: bool) {
	}

	pub fn think(&mut self, _s: &mut EditorEditState) {
	}
}
