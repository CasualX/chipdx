use super::*;

const LEVELSETS_PER_PAGE: usize = 14;

#[derive(Default)]
pub struct LevelSetMenu {
	pub selected: usize,
	pub items: Vec<String>,
	pub external_loader_label: Option<String>,
	pub splash: Vec<Option<shade::AnimatedTexture2D>>,
	pub ntime: i32,
	offset: usize,
	offsetf: f32,
}

impl LevelSetMenu {
	pub fn new(selected: usize, items: Vec<String>, external_loader_label: Option<String>, splash: Vec<Option<shade::AnimatedTexture2D>>) -> Self {
		let mut menu = Self { selected, items, external_loader_label, splash, ..Default::default() };
		menu.selected = menu.selected.min(menu.item_count().saturating_sub(1));
		menu.offset = menu.selected.saturating_sub(LEVELSETS_PER_PAGE / 2 - 1).min(menu.max_offset());
		menu.offsetf = menu.offset as f32;
		menu
	}

	fn item_count(&self) -> usize {
		self.items.len() + usize::from(self.external_loader_label.is_some())
	}

	fn max_offset(&self) -> usize {
		self.item_count().saturating_sub(LEVELSETS_PER_PAGE)
	}

	fn jump(&mut self, jump: isize, events: &mut Vec<MenuEvent>) {
		let selected = self.selected.saturating_add_signed(jump).min(self.item_count().saturating_sub(1));
		if selected != self.selected {
			self.selected = selected;
			events.push(MenuEvent::CursorMove);
			if selected <= self.offset {
				self.offset = selected.saturating_sub(1).min(self.max_offset());
			}
			else if selected >= self.offset + LEVELSETS_PER_PAGE - 1 {
				self.offset = (selected + 2).saturating_sub(LEVELSETS_PER_PAGE).min(self.max_offset());
			}
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
		if (input.a.is_pressed() || input.start.is_pressed()) && self.item_count() > 0 {
			if self.selected < self.items.len() {
				events.push(MenuEvent::LoadLevelSet { index: self.selected });
			}
			else {
				events.push(MenuEvent::LoadExternalLevelSet);
			}
			events.push(MenuEvent::CursorSelect);
		}
		if input.b.is_pressed() {
			events.push(MenuEvent::CloseMenu);
		}
	}
	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &Resources) {
		self.ntime += 1;

		let mut pool = shade::im::DrawPool::new();

		if let Some(Some(splash)) = self.splash.get(self.selected) {
			let cv = pool.get::<UiVertex, UiUniform>();
			cv.blend_mode = shade::BlendMode::Alpha;
			cv.shader = Some(resx.uishader.as_ref());

			let rect = resx.viewport.cast();
			cv.uniform.transform = Transform2f::ortho(rect);

			let time = self.ntime as f64 / 60.0;
			cv.uniform.texture = splash.get_frame(time).unwrap_or(&shade::DefaultTexture2D);

			let ss = resx.viewport.size();
			let color = [128, 128, 128, 255];
			let sprite = shade::d2::Sprite {
				bottom_left: UiVertex { pos: Vec2f(0.0, 0.0), uv: Vec2f(0.0, 1.0), color },
				bottom_right: UiVertex { pos: Vec2f(ss.x as f32, 0.0), uv: Vec2f(1.0, 1.0), color },
				top_left: UiVertex { pos: Vec2f(0.0, ss.y as f32), uv: Vec2f(0.0, 0.0), color },
				top_right: UiVertex { pos: Vec2f(ss.x as f32, ss.y as f32), uv: Vec2f(1.0, 0.0), color },
			};
			let rc = Bounds2!(0.0, 0.0, ss.x as f32, ss.y as f32);
			let height = splash.height as f32 * (ss.x as f32 / splash.width as f32);
			let [_, rc, _] = draw::flexv(rc, None, layout::Justify::Center, &[layout::Unit::Fr(1.0), layout::Unit::Abs(height), layout::Unit::Fr(1.0)]);
			// let [_, rc, _] = draw::flexh(rc, None, layout::Justify::Center, &[layout::Unit::Fr(1.0), layout::Unit::Abs(splash.width as f32), layout::Unit::Fr(1.0)]);
			cv.sprite_rect(&sprite, &rc);
		}


		let buf = pool.get::<shade::d2::TextVertex, shade::d2::TextUniform>();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let [top, _] = draw::flexv(rect, None, layout::Justify::Center, &[layout::Unit::Fr(1.0), layout::Unit::Fr(4.0)]);

		{
			let size = resx.font_size();

			let scribe = shade::d2::Scribe {
				font_size: size,
				line_height: size * (5.0 / 4.0),
				color: Vec4(255, 255, 255, 255),
				..Default::default()
			};

			buf.text_box(&resx.font, &scribe, &top, shade::d2::TextAlign::MiddleCenter, "Choose LevelSet");
		}

		let mut items = self.items.iter().map(|s| s as &dyn fmt::Display).collect::<Vec<_>>();
		if let Some(label) = &self.external_loader_label {
			items.push(label as &dyn fmt::Display);
		}

		let size = resx.font_size();
		let mut scribe = shade::d2::Scribe {
			font_size: size * 0.75,
			line_height: size * 0.75 * (5.0 / 4.0),
			..Default::default()
		};

		self.offsetf = Vec2(self.offsetf, 0.0).exp_decay(Vec2(self.offset as f32, 0.0), 15.0, 1.0 / 60.0).x;

		let top_opaque = rect.mins.y + size * 5.0;
		let bottom_opaque = top_opaque + (LEVELSETS_PER_PAGE - 1) as f32 * scribe.line_height;
		let top_transparent = top_opaque - scribe.line_height * 1.5;
		let bottom_transparent = bottom_opaque + scribe.line_height * 1.5;
		// Keep short collections centered within the list area.
		let padding = LEVELSETS_PER_PAGE.saturating_sub(items.len()) as f32 * scribe.line_height * 0.5;
		let mut y = top_opaque + padding - self.offsetf * scribe.line_height;

		for (i, item) in items.iter().enumerate() {
			if y > bottom_transparent {
				break;
			}
			if y >= top_transparent {
				let alpha = ((y - top_transparent) / (top_opaque - top_transparent))
					.min((bottom_transparent - y) / (bottom_transparent - bottom_opaque))
					.clamp(0.0, 1.0);
				let alpha = (alpha * 255.0) as u8;
				let color = if i == self.selected { 255 } else { 128 };
				scribe.color = Vec4(color, color, color, alpha);
				scribe.outline.w = alpha;

				let row = Bounds2::point(Vec2(rect.center().x, y), Vec2::ZERO);
				buf.text_lines(&resx.font, &scribe, &row, shade::d2::TextAlign::MiddleCenter, std::slice::from_ref(item));
			}
			y += scribe.line_height;
		}

		pool.draw(g);
	}
}
