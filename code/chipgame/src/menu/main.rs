use super::*;

#[derive(Default)]
pub struct MainMenu {
	pub title: String,
	pub selected: u8,
}

impl MainMenu {
	const ITEMS: [&'static str; 7] = [
		"New Game",
		"Continue",
		"Go to Level",
		"Options",
		"About LevelSet",
		"Switch LevelSet",
		"Exit",
	];
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		if input.up.is_pressed() {
			if self.selected > 0 {
				self.selected = self.selected - 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.down.is_pressed() {
			if self.selected < Self::ITEMS.len() as u8 - 1 {
				self.selected = self.selected + 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.a.is_pressed() {
			let evt = match self.selected {
				0 => MenuEvent::NewGame,
				1 => MenuEvent::Continue,
				2 => MenuEvent::OpenGoToLevel,
				3 => MenuEvent::OpenOptions,
				4 => MenuEvent::OpenAbout,
				5 => MenuEvent::SwitchLevelSet,
				_ => MenuEvent::ExitGame,
			};
			events.push(evt);
			events.push(MenuEvent::CursorSelect);
		}
	}
	pub fn draw(&mut self, g: &mut shade::Graphics, resx: &Resources) {
		let mut buf = shade::d2::TextBuffer::new();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let [top, bottom, _] = draw::flexv(rect, None, layout::Justify::Center, &[layout::Unit::Fr(1.0), layout::Unit::Fr(3.0), layout::Unit::Fr(1.0)]);

		{
			let size = resx.viewport.height() as f32 * FONT_SIZE;

			let scribe = shade::d2::Scribe {
				font_size: size,
				line_height: size * (5.0 / 4.0),
				color: Vec4(255, 255, 255, 255),
				..Default::default()
			};

			buf.text_box(&resx.font, &scribe, &top, shade::d2::TextAlign::MiddleCenter, &self.title);
		}

		draw::DrawMenuItems {
			items_text: &wrap_items(&Self::ITEMS),
			selected_index: self.selected as usize,
		}.draw(&mut buf, &bottom, resx);

		buf.draw(g);
	}
}
