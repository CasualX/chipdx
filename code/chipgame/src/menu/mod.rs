//! Menu system.

use std::{fmt, mem};
use cvmath::*;
use shade::d2::layout;
use crate::fx::Resources;
use crate::play;

mod draw;
mod event;
mod input;
mod levelset;
mod main;
mod menustate;
mod gamewin;
mod gameover;
mod pause;
mod options;
mod gotolevel;
mod unlocklevel;
mod about;
mod scout;
mod geometry;

pub use self::event::*;
pub use self::levelset::*;
use self::input::*;
pub use self::main::*;
pub use self::menustate::*;
pub use self::gamewin::*;
pub use self::gameover::*;
pub use self::pause::*;
pub use self::options::*;
pub use self::gotolevel::*;
pub use self::unlocklevel::*;
pub use self::about::*;
pub use self::scout::*;
pub use self::geometry::*;

const FONT_SIZE: f32 = 1.0 / 20.0;

pub fn darken(g: &mut shade::Graphics, resx: &Resources, alpha: u8) {
	let mut cv = shade::im::DrawBuilder::<UiVertex, UiUniform>::new();

	cv.blend_mode = shade::BlendMode::Alpha;
	cv.shader = Some(resx.colorshader.as_ref());

	let paint = shade::d2::Paint {
		template: UiVertex { pos: Vec2::ZERO, uv: Vec2::ZERO, color: [0, 0, 0, alpha] },
	};
	cv.fill_rect(&paint, &Bounds2!(-1.0, 1.0, 1.0, -1.0));

	cv.draw(g);
}

fn wrap_items<'a, const N: usize>(items: &'a [&'a str; N]) -> [&'a (dyn fmt::Display + 'a); N] {
	items.each_ref().map(|item| item as &dyn fmt::Display)
}

struct MenuItems<'a> {
	labels: &'a [&'a (dyn fmt::Display + 'a)],
	events: &'a [MenuEvent],
}

#[repr(transparent)]
pub struct FmtRealtime(pub f32);

impl fmt::Display for FmtRealtime {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let &FmtRealtime(realtime) = self;

		let realmins = (realtime as i32) / 60;
		let realsecs = (realtime as i32) % 60;
		let realmilis = realtime.fract() * 1000.0;

		if realmins > 0 {
			write!(f, "{}:{:02}.{:03}", realmins, realsecs, realmilis as i32)
		}
		else {
			write!(f, "{}.{:03}", realsecs, realmilis as i32)
		}
	}
}

pub struct PlayMetrics<'a> {
	pub level_number: i32,
	pub level_name: &'a str,
	pub attempts: i32,
	pub time: i32,
	pub realtime: f32,
	pub steps: i32,
	pub step_offset: i32,
	pub bonks: i32,
}

impl<'a> PlayMetrics<'a> {
	pub fn draw(&self, g: &mut shade::Graphics, resx: &Resources) {
		let text = fmtools::format!(
			"Level "{self.level_number}": \x1b[color=#ff0]"{self.level_name}"\x1b[color=#fff]\n"
			"Attempt: \x1b[color=#0f8]"{self.attempts}"\x1b[color=#fff]\n"
			"Real: \x1b[color=#0f8]"{FmtRealtime(self.realtime)}"\x1b[color=#fff]\n"
			"Time: \x1b[color=#0f8]"{chipcore::FmtTime(self.time)}"\x1b[color=#fff]\n"
			"Step offset: \x1b[color=#0f8]"{self.step_offset}"\x1b[color=#fff]\n"
			"Steps: \x1b[color=#0f8]"{self.steps}"\x1b[color=#fff]\n"
			"Bonks: \x1b[color=#0f8]"{self.bonks}"\x1b[color=#fff]\n"
		);

		draw_overlay(g, resx, shade::d2::TextAlign::MiddleLeft, &text);
	}
}

pub fn draw_metrics(g: &mut shade::Graphics, resx: &Resources, metrics: &shade::DrawMetrics) {
	let text = fmtools::format!(
		"Draw duration: \x1b[color=#0f8]"{metrics.draw_duration.as_secs_f64() * 1000.0:.2}"\x1b[color=#fff] ms\n"
		"Draw calls: \x1b[color=#0f8]"{metrics.draw_call_count}"\x1b[color=#fff]\n"
		"Vertex count: \x1b[color=#0f8]"{metrics.vertex_count}"\x1b[color=#fff]\n"
		"Bytes uploaded: \x1b[color=#0f8]"{metrics.bytes_uploaded as f64 / 1024.0:.2}"\x1b[color=#fff] KiB\n"
	);

	draw_overlay(g, resx, shade::d2::TextAlign::BottomLeft, &text);
}

pub fn draw_overlay(g: &mut shade::Graphics, resx: &Resources, align: shade::d2::TextAlign, text: &str) {
	let mut buf = shade::d2::TextBuffer::new();
	buf.blend_mode = shade::BlendMode::Alpha;
	buf.shader = Some(&*resx.font.shader);

	let rect = resx.viewport.cast();
	buf.uniform.transform = cvmath::Transform2f::ortho(rect);
	buf.uniform.texture = &*resx.font.texture;

	let size = rect.height() * FONT_SIZE * 0.75;

	let scribe = shade::d2::Scribe {
		font_size: size,
		line_height: size * (5.0 / 4.0),
		color: Vec4(255, 255, 255, 255),
		..Default::default()
	};

	let [_, rect, _] = draw::flexh(rect, None, layout::Justify::Start, &[layout::Unit::Pct(2.5), layout::Unit::Fr(1.0), layout::Unit::Pct(2.5)]);
	let [_, rect, _] = draw::flexv(rect, None, layout::Justify::Start, &[layout::Unit::Pct(2.5), layout::Unit::Fr(1.0), layout::Unit::Pct(2.5)]);

	buf.text_box(&resx.font, &scribe, &rect, align, &text);

	buf.draw(g);
}
