use super::*;

use crate::menu::{UiUniform, UiVertex};

impl FxState {
	pub(super) fn render_ui(&mut self, g: &mut shade::Graphics, resx: &Resources, time: f64) {
		let darken_time = f32::min(1.0, (time - self.darken_time) as f32 / 0.2);
		let alpha = if self.darken { darken_time } else { 1.0 - darken_time };
		if alpha > 0.0 {
			let alpha = (alpha * 168.0).round() as u8;
			crate::menu::darken(g, resx, alpha);
		}

		let mut pool = shade::im::DrawPool::new();

		let cv = pool.get::<UiVertex, UiUniform>();
		cv.blend_mode = shade::BlendMode::Alpha;
		cv.shader = Some(resx.uishader.as_ref());

		cv.uniform.transform = Transform2f::ortho(resx.viewport.cast());
		cv.uniform.texture = resx.menubg.as_ref();

		// Draw the backgrounds for various UI elements
		let paint = shade::d2::Paint {
			template: UiVertex {
				pos: Vec2f::ZERO,
				uv: Vec2f::ZERO,
				color: [255, 255, 255, 64],
			},
		};
		let ss = resx.viewport.size();
		let a = f32::max(ss.y as f32 * 0.075, ss.x as f32 * 0.05);
		let y = resx.viewport.bottom() as f32 - a * 1.1;
		let ref game = self.game;

		let [[keys_x1, keys_x2], [items_x1, items_x2]] = shade::d2::layout::flex1d(resx.viewport.left() as f32, resx.viewport.right() as f32, None, shade::d2::layout::Justify::SpaceAround, &[
			shade::d2::layout::Unit::Abs(a * 5.0),
			shade::d2::layout::Unit::Abs(a * 5.0),
		]);
		let [[chips_x1, chips_x2], [time_x1, time_x2]] = shade::d2::layout::flex1d(resx.viewport.left() as f32, resx.viewport.right() as f32, None, shade::d2::layout::Justify::SpaceAround, &[
			shade::d2::layout::Unit::Abs(a * 3.0),
			shade::d2::layout::Unit::Abs(a * 3.0),
		]);
		if game.ps.keys.iter().any(|&k| k > 0) {
			cv.fill_rect(&paint, &Bounds2!(keys_x1, y, keys_x2, resx.viewport.bottom() as f32));
		}
		if game.ps.boots != chipcore::Boots::NONE {
			cv.fill_rect(&paint, &Bounds2!(items_x1, y, items_x2, resx.viewport.bottom() as f32));
		}
		if !self.darken {
			cv.fill_rect(&paint, &Bounds2!(chips_x1, 0.0, chips_x2, a * 0.9));
			cv.fill_rect(&paint, &Bounds2!(time_x1, 0.0, time_x2, a * 0.9));
		}

		// Draw the inventory items
		let sprites = pool.get::<shade::d2::TexturedVertex, shade::d2::TexturedUniform>();
		sprites.blend_mode = shade::BlendMode::Alpha;
		sprites.shader = Some(resx.shader2d_pixelart.as_ref());
		sprites.uniform.transform = Transform2f::ortho(resx.viewport.cast());
		sprites.uniform.texture = resx.spritesheet_texture.as_ref();
		if game.ps.keys[0] > 0 {
			draw_sprite(sprites, resx, chipty::SpriteId::BlueKey, Vec2(keys_x1 + a * 0.5, y), a);
		}
		if game.ps.keys[1] > 0 {
			draw_sprite(sprites, resx, chipty::SpriteId::RedKey, Vec2(keys_x1 + a * 1.5, y), a);
		}
		if game.ps.keys[2] > 0 {
			draw_sprite(sprites, resx, chipty::SpriteId::GreenKey, Vec2(keys_x1 + a * 2.5, y), a);
		}
		if game.ps.keys[3] > 0 {
			draw_sprite(sprites, resx, chipty::SpriteId::YellowKey, Vec2(keys_x1 + a * 3.5, y), a);
		}
		if game.ps.boots.has(chipcore::Boots::FLIPPERS) {
			draw_sprite(sprites, resx, chipty::SpriteId::Flippers, Vec2(items_x1 + a * 0.5, y), a);
		}
		if game.ps.boots.has(chipcore::Boots::FIRE_BOOTS) {
			draw_sprite(sprites, resx, chipty::SpriteId::FireBoots, Vec2(items_x1 + a * 1.5, y), a);
		}
		if game.ps.boots.has(chipcore::Boots::ICE_SKATES) {
			draw_sprite(sprites, resx, chipty::SpriteId::IceSkates, Vec2(items_x1 + a * 2.5, y), a);
		}
		if game.ps.boots.has(chipcore::Boots::SUCTION_BOOTS) {
			draw_sprite(sprites, resx, chipty::SpriteId::SuctionBoots, Vec2(items_x1 + a * 3.5, y), a);
		}

		// Draw the CHIPS and TIME counters
		{
			let tbuf = pool.get::<shade::d2::TextVertex, shade::d2::TextUniform>();
			tbuf.shader = Some(&*resx.font.shader);

			let transform = Transform2f::ortho(resx.viewport.cast());
			tbuf.uniform = shade::d2::TextUniform {
				transform,
				texture: &*resx.font.texture,
				outline_width_absolute: 0.8,
				unit_range: Vec2::dup(4.0f32) / Vec2(232.0f32, 232.0f32),
				..Default::default()
			};
			let size = ss.y as f32 * 0.025;
			let mut scribe = shade::d2::Scribe {
				font_size: size,
				line_height: size,
				outline: Vec4(0, 0, 0, 255),
				..Default::default()
			};
			for i in 0..game.ps.keys.len() {
				if game.ps.keys[i] >= 2 {
					scribe.color = match i {
						0 => Vec4(0, 255, 255, 255),
						1 => Vec4(255, 0, 0, 255),
						2 => Vec4(0, 255, 0, 255),
						3 => Vec4(255, 255, 0, 255),
						_ => Vec4(255, 255, 255, 255),
					};
					tbuf.text_box(&resx.font, &scribe, &Bounds2!(keys_x1 + a * (i as f32 + 0.5), y + 0.0, keys_x1 + a * (i as f32 + 1.5), y + a), shade::d2::TextAlign::BottomCenter, &format!("x{}", game.ps.keys[i]));
				}
			}

			scribe.font_size = a * 0.5;
			scribe.line_height = scribe.font_size * 1.2;

			let chips_x = (keys_x1 + keys_x2) * 0.5;
			let chips_remaining = i32::max(0, game.field.required_chips - game.ps.chips);
			let time_x = (items_x1 + items_x2) * 0.5;
			let time_remaining = if game.field.time_limit <= 0 { -1 } else { f32::ceil((game.field.time_limit * 60 - game.time) as f32 / 60.0) as i32 };

			scribe.color = Vec4(255, 0, 128, 255);
			tbuf.text_box(&resx.font, &scribe, &Bounds2!(chips_x, 0.0, chips_x, a), shade::d2::TextAlign::TopRight, "💎");
			scribe.color = if chips_remaining <= 0 { Vec4::unpack8(0xFF00FFFF) } else { Vec4::unpack8(0xFF00FF00) };
			tbuf.text_box(&resx.font, &scribe, &Bounds2!(chips_x + a * 0.125, 0.0, chips_x, a), shade::d2::TextAlign::TopLeft, &format!("{chips_remaining}"));

			scribe.color = Vec4(255, 0, 128, 255);
			tbuf.text_box(&resx.font, &scribe, &Bounds2!(time_x, 0.0, time_x, a), shade::d2::TextAlign::TopRight, "⏰");
			scribe.color = if time_remaining <= 0 { Vec4::unpack8(0xFF00FFFF) } else { Vec4::unpack8(0xFF00FF00) };
			if time_remaining >= 0 {
				tbuf.text_box(&resx.font, &scribe, &Bounds2!(time_x + a * 0.125, 0.0, time_x + a * 0.125, a), shade::d2::TextAlign::TopLeft, &format!("{time_remaining}"));
			}
			else {
				tbuf.text_box(&resx.font, &scribe, &Bounds2!(time_x + a * 0.125, 0.0, time_x, a), shade::d2::TextAlign::TopLeft, "- - -");
			}
		}

		// Draw the level title or hint text
		let mut darken = false;
		if self.game.time == 0 {
			darken = true;
			let tbuf = pool.get::<shade::d2::TextVertex, shade::d2::TextUniform>();
			tbuf.shader = Some(&*resx.font.shader);

			let transform = Transform2f::ortho(resx.viewport.cast());
			tbuf.uniform = shade::d2::TextUniform {
				transform,
				texture: &*resx.font.texture,
				outline_width_absolute: 0.8,
				unit_range: Vec2::dup(4.0f32) / Vec2(232.0f32, 232.0f32),
				..Default::default()
			};
			let size = ss.y as f32 * 0.05;
			let mut scribe = shade::d2::Scribe {
				font_size: size,
				line_height: size,
				color: Vec4(255, 255, 255, 255),
				outline: Vec4(0, 0, 0, 255),
				..Default::default()
			};
			let level_index = format!("~ Level {} ~", self.level_number);
			let (width, _) = scribe.measure_text(&resx.font.font, &level_index);
			tbuf.text_write(&resx.font, &mut scribe, &mut shade::d2::Cursor(Vec2((ss.x as f32 - width) * 0.5, ss.y as f32 * 0.75 - size * 1.2)), &level_index);
			let (width, _) = scribe.measure_text(&resx.font.font, &self.game.field.name);
			scribe.color = Vec4(255, 255, 0, 255);
			tbuf.text_write(&resx.font, &mut scribe, &mut shade::d2::Cursor(Vec2((ss.x as f32 - width) * 0.5, ss.y as f32 * 0.75)), &self.game.field.name);
			if let Some(password) = &self.game.field.password {
				let password = format!("Password: {}", password);
				let (width, _) = scribe.measure_text(&resx.font.font, &password);
				tbuf.text_write(&resx.font, &mut scribe, &mut shade::d2::Cursor(Vec2((ss.x as f32 - width) * 0.5, ss.y as f32 * 0.75 + size * 1.2)), &password);
			}
		}
		else if matches!(self.game.time_state, chipcore::TimeState::Running) && self.game.is_show_hint() {
			if let Some(hint) = &self.game.field.hint {
				darken = true;

				let tbuf = pool.get::<shade::d2::TextVertex, shade::d2::TextUniform>();
				tbuf.shader = Some(&*resx.font.shader);

				let rect = resx.viewport.cast();
				let max_hint_width = rect.size().x * 0.9;
				let hpad = (rect.size().x - max_hint_width) * 0.5;
				let hint_rect = Bounds2!(rect.mins.x + hpad, rect.mins.y, rect.maxs.x - hpad, rect.maxs.y);
				let transform = Transform2f::ortho(rect);
				tbuf.uniform = shade::d2::TextUniform {
					transform,
					texture: &*resx.font.texture,
					outline_width_absolute: 0.8,
					unit_range: Vec2::dup(4.0f32) / Vec2(232.0f32, 232.0f32),
					..Default::default()
				};
				let size = ss.y as f32 * 0.05;
				let mut scribe = shade::d2::Scribe {
					font_size: size,
					line_height: size * 1.2,
					color: Vec4(255, 255, 255, 255),
					outline: Vec4(0, 0, 0, 255),
					..Default::default()
				};

				// Temporarily adjust font size if hint is too wide
				// TODO: Implement word-wrapping
				let (hint_width, _) = scribe.measure_text(&resx.font.font, &hint);
				if hint_width > max_hint_width {
					// Hint is too wide, use a smaller font size but keep a screen edge margin
					scribe.font_size = size * max_hint_width / hint_width;
					scribe.line_height = scribe.font_size * 1.2;
				}

				tbuf.text_box(&resx.font, &scribe, &hint_rect, shade::d2::TextAlign::MiddleCenter, &hint);
			}
		}

		if self.darken != darken {
			self.darken = darken;
			self.darken_time = time;
		}

		pool.draw(g);
	}
}

fn draw_sprite(cv: &mut shade::d2::TexturedBuffer, resx: &Resources, sprite: chipty::SpriteId, pos: Vec2<f32>, size: f32) {
	let uv = sprite_uv(&resx.spritesheet_meta, sprite, 0);
	let color = Vec4(255, 255, 255, 255);
	let top_left = shade::d2::TexturedTemplate { uv: uv.top_left(), color };
	let bottom_left = shade::d2::TexturedTemplate { uv: uv.bottom_left(), color };
	let top_right = shade::d2::TexturedTemplate { uv: uv.top_right(), color };
	let bottom_right = shade::d2::TexturedTemplate { uv: uv.bottom_right(), color };
	let sprite = shade::d2::Sprite { bottom_left, top_left, top_right, bottom_right };
	cv.sprite_rect(&sprite, &Bounds2(pos, pos + Vec2(size, size)));
}

fn sprite_uv(sheet: &chipty::SpriteSheet<chipty::SpriteId>, sprite: chipty::SpriteId, frame: usize) -> Bounds2f {
	let entry = sheet.sprites.get(&sprite).unwrap();
	assert!(frame < entry.len as usize, "frame index in bounds");
	let f = &sheet.frames[(entry.index as usize) + frame];
	let [x, y, width, height] = f.rect;
	Bounds2!(x as f32 / sheet.width as f32, y as f32 / sheet.height as f32, (x + width) as f32 / sheet.width as f32, (y + height) as f32 / sheet.height as f32)
}
