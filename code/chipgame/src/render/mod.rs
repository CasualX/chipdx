//! Renderer.

use std::collections::HashMap;
use cvmath::*;

use crate::fx::Resources;

mod animation;
mod effect;
mod object;
mod objectmap;
mod render;
mod renderstate;

pub use self::animation::*;
pub use self::effect::*;
pub use self::object::*;
pub use self::objectmap::*;
pub use self::render::*;
pub use self::renderstate::*;

#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TileGfx {
	pub sprite: chipty::SpriteId,
	pub model: chipty::ModelId,
}

pub type TileGfxFn = fn(chipty::Terrain) -> TileGfx;

pub fn drawbg(g: &mut dyn shade::IGraphics, resx: &Resources) {
	g.begin(&shade::BeginArgs::Immediate {
		viewport: resx.viewport,
		color: &[resx.backcolor()],
		levels: None,
		depth: Some(resx.backdepth()),
	});
	let mut cv = shade::im::DrawBuilder::<render::Vertex, render::Uniform>::new();
	cv.depth_test = None;
	cv.cull_mode = None;
	cv.shader = Some(resx.shader.as_ref());
	cv.uniform.texture = resx.menubg.as_ref();
	let info = resx.menubg.as_ref().info();
	let tex_w = info.width as f32;
	let tex_h = info.height as f32;
	let vp_w = resx.viewport.width() as f32;
	let vp_h = resx.viewport.height() as f32;
	// Number of times the texture should repeat across the screen.
	let repeat_x = vp_w / (tex_w * resx.menubg_scale);
	let repeat_y = vp_h / (tex_h * resx.menubg_scale);
	// In pixel units (vertex shader divides by texture size)
	let u_max = tex_w * repeat_x;
	let v_max = tex_h * repeat_y;
	{
		let mut p = cv.begin(shade::PrimType::Triangles, 4, 2);
		p.add_indices_quad();
		p.add_vertices(&[
			// Note: Y flipped like original (bottom uses v_max, top uses 0.0)
			render::Vertex { pos: cvmath::Vec3(-1.0, -1.0, 0.0), uv: cvmath::Vec2(0.0, v_max), color: [255; 4] },
			render::Vertex { pos: cvmath::Vec3( 1.0, -1.0, 0.0), uv: cvmath::Vec2(u_max, v_max), color: [255; 4] },
			render::Vertex { pos: cvmath::Vec3( 1.0,  1.0, 0.0), uv: cvmath::Vec2(u_max, 0.0),  color: [255; 4] },
			render::Vertex { pos: cvmath::Vec3(-1.0,  1.0, 0.0), uv: cvmath::Vec2(0.0, 0.0),   color: [255; 4] },
		]);
	}
	cv.draw(g);
	g.clear(&shade::ClearArgs { depth: Some(1.0), ..Default::default() });
	g.end();
}

struct SpriteUV {
	top_left: Vec2f,
	top_right: Vec2f,
	bottom_left: Vec2f,
	bottom_right: Vec2f,
	width: f32,
	height: f32,
	origin: Vec2f,
}

fn vec2f(v: Vec2i) -> Vec2f {
	Vec2f::new(v.x as f32, v.y as f32)
}

fn sprite_uv(sheet: &shade::atlas::Atlas<chipty::SpriteId>, sprite: chipty::SpriteId, frame: u16) -> SpriteUV {
	let Some(entry) = sheet.sprites.get(&sprite) else {
		panic!("sprite {:?} not found in sheet", sprite);
	};
	let frame = entry.get_frame_wrapping(frame as usize)
		.unwrap_or_else(|| panic!("sprite {:?} has zero frames", sprite));
	let rect = frame.rect;
	let quad = frame.get_sprite();

	SpriteUV {
		top_left: vec2f(quad.top_left),
		top_right: vec2f(quad.top_right),
		bottom_left: vec2f(quad.bottom_left),
		bottom_right: vec2f(quad.bottom_right),
		width: rect.width as f32,
		height: rect.height as f32,
		origin: Vec2f::new(frame.origin.x as f32, frame.origin.y as f32),
	}
}
