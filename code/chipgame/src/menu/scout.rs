use super::*;

#[derive(Default)]
pub struct ScoutMode {
	up: bool,
	down: bool,
	left: bool,
	right: bool,
}

impl ScoutMode {
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		if input.select.is_pressed() {
			events.push(MenuEvent::ResumePlay);
		}
		if input.start.is_pressed() {
			events.push(MenuEvent::CloseMenu);
			events.push(MenuEvent::OpenPauseMenu);
		}

		self.up = input.up.is_held();
		self.down = input.down.is_held();
		self.left = input.left.is_held();
		self.right = input.right.is_held();
	}
	pub fn draw(&mut self, g: &mut shade::Graphics, resx: &Resources) {
		let mut buf = shade::d2::TextBuffer::new();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let size = resx.font_size() * 0.75;

		let scribe = shade::d2::Scribe {
			font_size: size,
			line_height: size * (5.0 / 4.0),
			color: Vec4(255, 255, 255, 255),
			..Default::default()
		};

		// buf.text_box(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleCenter, "Paused");
		buf.text_box(&resx.font, &scribe, &rect, shade::d2::TextAlign::TopCenter, if !self.up { "▲" } else { "△" });
		buf.text_box(&resx.font, &scribe, &rect, shade::d2::TextAlign::BottomCenter, if !self.down { "▼" } else { "▽" });
		buf.text_box(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleLeft, if !self.left { "◀" } else { "◁" });
		buf.text_box(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleRight, if !self.right { "▶" } else { "▷" });

		buf.draw(g);
	}
}
