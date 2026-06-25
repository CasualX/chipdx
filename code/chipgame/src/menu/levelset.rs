use super::*;

#[derive(Default)]
pub struct LevelSetMenu {
	pub selected: usize,
	pub items: Vec<String>,
	pub splash: Vec<Option<shade::AnimatedTexture2D>>,
	pub ntime: i32,
}

impl LevelSetMenu {
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		if input.up.is_pressed() {
			if self.selected > 0 {
				self.selected = self.selected - 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.down.is_pressed() {
			if self.selected + 1 < self.items.len() {
				self.selected = self.selected + 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.a.is_pressed() {
			events.push(MenuEvent::LoadLevelSet { index: self.selected });
			events.push(MenuEvent::CursorSelect);
		}
		if input.b.is_pressed() {
			events.push(MenuEvent::CloseMenu);
		}
	}
	pub fn draw(&mut self, g: &mut shade::Graphics, resx: &Resources) {
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

		let [top, bottom, _] = draw::flexv(rect, None, layout::Justify::Center, &[layout::Unit::Fr(1.0), layout::Unit::Fr(3.0), layout::Unit::Fr(1.0)]);

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

		let items = self.items.iter().map(|s| s as &dyn fmt::Display).collect::<Vec<_>>();

		draw::DrawMenuItems {
			items_text: &items,
			selected_index: self.selected,
		}.draw(buf, &bottom, resx);

		pool.draw(g);
	}
}
