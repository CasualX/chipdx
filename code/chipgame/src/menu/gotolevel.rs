use super::*;

const LEVELS_PER_PAGE: i32 = 14;

fn clip_offset(offset: i32, len: i32) -> i32 {
	i32::max(0, i32::min(len - LEVELS_PER_PAGE, offset))
}

struct LevelEntry {
	level_number: i32,
	progress: chipty::LevelProgress,
	label: String,
	trophies: Option<draw::DrawTrophies>,
}

#[derive(Default)]
pub struct GoToLevel {
	selected: i32,
	offset: i32,
	offsetf: f32,
	items: Vec<LevelEntry>,
}

impl GoToLevel {
	pub fn load_items(&mut self, level_set: &play::LevelSet, save_data: &play::SaveData) {
		self.items.clear();
		// Dummy entry for "Unlock level"
		self.items.push(LevelEntry {
			level_number: 0,
			progress: chipty::LevelProgress::Completed,
			label: "\x1b[draw_mask=0]🌟\x1b[draw_mask=1] Unlock level".to_string(),
			trophies: None,
		});
		for level_index in 0..save_data.unlocked_levels.len() {
			let level_number = (level_index + 1) as i32;
			let Some(level) = level_set.levels.get(level_index) else { continue };
			let Some(progress) = save_data.get_level_progress(level_number) else { continue };
			if save_data.current_level == level_number {
				self.selected = self.items.len() as i32;
			}
			let prefix = match progress {
				chipty::LevelProgress::Locked => "\x1b[draw_mask=0]🌟\x1b[draw_mask=1]",
				chipty::LevelProgress::Unlocked => "\x1b[draw_mask=0]🌟\x1b[draw_mask=1]",
				chipty::LevelProgress::Completed => "🌟",
			};
			self.items.push(LevelEntry {
				level_number,
				progress,
				label: format!("{prefix} Level {level_number}: {}", level.name),
				trophies: Some(draw::DrawTrophies::new(level_number, &level, save_data)),
			});
		}
		self.offset = clip_offset(self.selected - LEVELS_PER_PAGE / 2 + 1, self.items.len() as i32);
		self.offsetf = self.offset as f32;
	}

	fn jump(&mut self, jump: i32, events: &mut Vec<MenuEvent>) {
		let selected = i32::max(0, i32::min(self.items.len() as i32 - 1, self.selected + jump));
		if self.selected != selected {
			events.push(MenuEvent::CursorMove);
			self.selected = selected;
			if self.selected <= self.offset {
				self.offset = clip_offset(selected - 1, self.items.len() as i32);
			}
			else if self.selected >= self.offset + LEVELS_PER_PAGE - 1 {
				self.offset = clip_offset(selected - LEVELS_PER_PAGE + 2, self.items.len() as i32);
			}

			// Request the level preview
			let level_number = self.items[selected as usize].level_number;
			events.push(MenuEvent::PreviewLevel { level_number });
		}
	}

	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		if input.up.is_pressed() {
			self.jump(-1, events);
		}
		if input.left.is_pressed() {
			self.jump(-10, events);
		}
		if input.down.is_pressed() {
			self.jump(1, events);
		}
		if input.right.is_pressed() {
			self.jump(10, events);
		}
		if input.a.is_pressed() || input.start.is_pressed() {
			let evt = match self.selected {
				0 => MenuEvent::OpenUnlockLevel,
				index => MenuEvent::PlayLevel { level_number: self.items[index as usize].level_number },
			};
			events.push(MenuEvent::CursorSelect);
			events.push(MenuEvent::PreviewExit);
			events.push(evt);
		}
		if input.b.is_pressed() {
			events.push(MenuEvent::PreviewExit);
			events.push(MenuEvent::CloseMenu);
		}
	}
	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &Resources) {
		let mut buf = shade::d2::TextBuffer::new();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let size = resx.font_size();

		let [top, bottom] = draw::flexv(rect, None, layout::Justify::Center, &[layout::Unit::Fr(1.0), layout::Unit::Fr(4.0)]);
		let [_, panel, _] = draw::flexh(bottom, None, layout::Justify::Center, &[layout::Unit::Fr(8.0), layout::Unit::Abs(size * 10.0), layout::Unit::Fr(1.0)]);

		let scribe = shade::d2::Scribe {
			font_size: size,
			line_height: size * (5.0 / 4.0),
			color: Vec4(255, 255, 255, 255),
			..Default::default()
		};

		buf.text_box(&resx.font, &scribe, &top, shade::d2::TextAlign::MiddleCenter, "Go to level");

		let mut scribe = shade::d2::Scribe {
			font_size: size * 0.75,
			line_height: size * 0.75 / 32.0 * 40.0,
			..Default::default()
		};

		self.offsetf = Vec2(self.offsetf, 0.0).exp_decay(Vec2(self.offset as f32, 0.0), 15.0, 1.0 / 60.0).x;

		let mut y = size * 5.0 - self.offsetf * scribe.line_height;
		let top_transparent = size * 5.0 - scribe.line_height * 1.5;
		let top_opaque = size * 5.0;
		let bottom_opaque = top_opaque + (LEVELS_PER_PAGE - 1) as f32 * scribe.line_height;
		let bottom_transparent = bottom_opaque + scribe.line_height * 1.5;

		for i in 0..self.items.len() as i32 {
			let item = &self.items[i as usize];

			if y < top_transparent {
				y += scribe.line_height;
				continue;
			}
			else if y > bottom_transparent {
				break;
			}

			let alpha = if y < top_opaque {
				(y - top_transparent) / (top_opaque - top_transparent)
			}
			else if y > bottom_opaque {
				(bottom_transparent - y) / (bottom_transparent - bottom_opaque)
			}
			else {
				1.0
			};
			let alpha = if matches!(item.progress, chipty::LevelProgress::Locked) && i != self.selected { alpha * 0.5 } else { alpha };
			let alpha = (f32::clamp(alpha, 0.0, 1.0) * 255.0) as u8;
			let color = if i == self.selected { 255 } else { 128 };
			scribe.color = Vec4(color, color, color, alpha);
			scribe.outline.w = alpha;

			let rect = Bounds2::point(Vec2(resx.viewport.width() as f32 * 0.125 * 0.75, y), Vec2::ZERO);
			buf.text_box(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleLeft, &item.label);

			y += scribe.line_height;
		}

		if let Some(trophies) = &self.items.get(self.selected as usize).and_then(|it| it.trophies.as_ref()) {
			trophies.draw(&mut buf, &panel, resx);
		}

		buf.draw(g);
	}
}
