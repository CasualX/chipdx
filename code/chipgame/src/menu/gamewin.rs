use super::*;

#[derive(Default)]
pub struct Fireworks {
	time_start: f64,
	pos: Vec2f,
}
impl Fireworks {
	fn draw(&mut self, g: &mut shade::Graphics, resx: &Resources, time: f64, rc: Bounds2f) {
		let mut rng = urandom::new();

		if self.time_start == 0.0 {
			self.time_start = time - rng.range(0.0..1.0);
		}

		if time >= self.time_start + 1.0 {
			self.time_start = time;
			self.pos.x = rng.range(rc.left()..rc.right()).round();
			self.pos.y = rng.range(rc.top()..rc.bottom()).round();
		}

		let atime = (time - self.time_start) as f32;
		if atime < 1.0 {
			// 12 frames of animation
			let t = f32::clamp(atime, 0.0, 1.0);
			let aindex = f32::floor(t * 13.0).min(12.0);

			let d_size = 96.0;
			let u = aindex * d_size;
			let v = d_size * 2.0;
			let texsize = Vec2f(1152.0, 288.0);

			let mut buf = shade::im::DrawBuilder::<UiVertex, UiUniform>::new();
			buf.blend_mode = shade::BlendMode::Alpha;
			buf.shader = Some(resx.uishader.as_ref());
			buf.uniform.transform = Transform2f::ortho(resx.viewport.cast());
			buf.uniform.texture = resx.effects.as_ref();
			let rc = Bounds2f::point(self.pos, Vec2f(d_size, d_size) * 0.5);
			let color = [255; 4];
			let sprite = shade::d2::Sprite {
				bottom_left: UiVertex { pos: Vec2f::ZERO, uv: Vec2f(u, v + d_size) / texsize, color },
				top_left: UiVertex { pos: Vec2f::ZERO, uv: Vec2f(u, v) / texsize, color },
				top_right: UiVertex { pos: Vec2f::ZERO, uv: Vec2f(u + d_size, v) / texsize, color },
				bottom_right: UiVertex { pos: Vec2f::ZERO, uv: Vec2f(u + d_size, v + d_size) / texsize, color },
			};
			buf.sprite_rect(&sprite, &rc);
			buf.draw(g);
		}

	}
}

#[derive(Default)]
pub struct GameWinMenu {
	pub selected: u8,
	pub level_number: i32,
	pub level_name: String,
	pub attempts: i32,
	pub time: i32,
	pub steps: i32,
	pub bonks: i32,
	pub time_high_score: i32,
	pub steps_high_score: i32,
	pub time_fireworks: Fireworks,
	pub steps_fireworks: Fireworks,
}

impl GameWinMenu {
	const ITEMS: [&'static str; 4] = ["Onward!", "Retry", "Save Replay", "Main Menu"];
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		if input.up.is_pressed() {
			if self.selected > 0 {
				self.selected -= 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.down.is_pressed() {
			if self.selected < Self::ITEMS.len() as u8 - 1 {
				self.selected += 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.a.is_pressed() {
			let evt = match self.selected {
				0 => MenuEvent::PlayNextLevel,
				1 => MenuEvent::RetryLevel,
				2 => MenuEvent::SaveReplay,
				_ => MenuEvent::OpenMainMenu,
			};
			events.push(evt);
			events.push(MenuEvent::CursorSelect);
		}
	}
	pub fn draw(&mut self, g: &mut shade::Graphics, resx: &Resources, time: f64) {
		let mut buf = shade::d2::TextBuffer::new();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let [top, middle, bottom] = draw::flexv(rect, None, layout::Justify::Start, &[layout::Unit::Fr(1.0); 3]);

		draw::DrawPlayTitle {
			level_number: self.level_number,
			level_name: &self.level_name,
			subtitle: Some(&"\x1b[color=#f08]Level Complete!")
		}.draw(&mut buf, &top, resx);

		let [_, middle, _] = draw::flexh(middle, None, layout::Justify::Center, &[layout::Unit::Fr(1.0); 3]);

		draw::DrawScoreCard {
			attempts: self.attempts,
			time: self.time,
			steps: self.steps,
			bonks: self.bonks,
			time_high_score: self.time_high_score,
			steps_high_score: self.steps_high_score,
		}.draw(&mut buf, &middle, resx);

		draw::DrawMenuItems {
			items_text: &wrap_items(&Self::ITEMS),
			selected_index: self.selected as usize,
		}.draw(&mut buf, &bottom, resx);

		buf.draw(g);

		let size = resx.font_size();
		if self.time_high_score < 0 || self.time < self.time_high_score {
			let mins = Vec2f(middle.right() - size * 3.0, middle.center().y - size);
			let maxs = Vec2f(middle.right(), middle.center().y);
			self.time_fireworks.draw(g, resx, time, Bounds2f(mins, maxs));
		}
		if self.steps_high_score < 0 || self.steps < self.steps_high_score {
			let mins = Vec2f(middle.right() - size, middle.center().y);
			let maxs = Vec2f(middle.right(), middle.center().y + size);
			self.steps_fireworks.draw(g, resx, time, Bounds2f(mins, maxs));
		}
	}
}
