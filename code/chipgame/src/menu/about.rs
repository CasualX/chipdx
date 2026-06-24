use super::*;

#[derive(Default)]
pub struct AboutMenu {
	pub text: String,
}

impl AboutMenu {
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		if input.b.is_pressed() || input.a.is_pressed() || input.start.is_pressed() {
			events.push(MenuEvent::CloseMenu);
		}
	}
	pub fn draw(&mut self, g: &mut shade::Graphics, resx: &Resources) {
		let mut buf = shade::d2::TextBuffer::new();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let size = rect.height() * FONT_SIZE * 0.75;

		let scribe = shade::d2::Scribe {
			font_size: size,
			line_height: size * (5.0 / 4.0),
			color: Vec4(255, 255, 255, 255),
			..Default::default()
		};

		let [_, rect] = draw::flexh(rect, None, layout::Justify::Start, &[layout::Unit::Pct(2.5), layout::Unit::Fr(1.0)]);

		buf.text_box(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleLeft, &self.text);

		buf.draw(g);
	}
}
