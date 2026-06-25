use super::*;

pub struct DrawMenuItems<'a> {
	pub items_text: &'a [&'a dyn fmt::Display],
	pub selected_index: usize,
}

impl<'a> DrawMenuItems<'a> {
	pub fn draw<'r>(&self, buf: &mut shade::d2::TextBuffer<'r>, rect: &Bounds2<f32>, resx: &'r Resources) {
		let size = resx.font_size();

		let mut scribe = shade::d2::Scribe {
			font_size: size,
			line_height: size * (5.0 / 4.0),
			..Default::default()
		};

		let mut y = (rect.height() - self.items_text.len() as f32 * scribe.line_height) * 0.5;

		for (i, line) in self.items_text.iter().enumerate() {
			scribe.color = if i == self.selected_index {
				Vec4(255, 255, 255, 255)
			}
			else {
				Vec4(128, 128, 128, 255)
			};

			let next_y = y + scribe.line_height;
			let rect = Bounds2!(rect.mins.x, rect.mins.y + y, rect.maxs.x, rect.mins.y + next_y);
			y = next_y;
			buf.text_lines(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleCenter, std::slice::from_ref(line));
		}
	}
}
