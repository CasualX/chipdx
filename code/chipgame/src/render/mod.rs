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

pub fn drawbg(g: &mut shade::Graphics, resx: &Resources) {
	g.begin(&shade::BeginArgs::Immediate {
		viewport: resx.viewport,
		color: &[resx.backcolor],
		levels: None,
		depth: resx.backdepth,
	});
	let mut cv = shade::im::DrawBuilder::<render::Vertex, render::Uniform>::new();
	cv.depth_test = None;
	cv.cull_mode = None;
	cv.shader = resx.shader;
	cv.uniform.texture = resx.menubg;
	let info = g.texture2d_get_info(resx.menubg).unwrap();
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

fn sprite_uv(sheet: &chipty::SpriteSheet<chipty::SpriteId>, sprite: chipty::SpriteId, frame: u16) -> SpriteUV {
	let Some(entry) = sheet.sprites.get(&sprite) else {
		panic!("sprite {:?} not found in sheet", sprite);
	};
	let index = entry.index as usize + if entry.len == 0 {
		panic!("sprite entry has zero frames");
	}
	else if entry.len == 1 {
		0
	}
	else {
		(frame as usize) % (entry.len as usize)
	};

	let frame = &sheet.frames[index];
	let [x, y, width, height] = frame.rect;

	let ax = x as f32;
	let ay = y as f32;
	let bx = (x + width) as f32;
	let cy = (y + height) as f32;

	let a = Vec2f::new(ax, ay);
	let b = Vec2f::new(bx, ay);
	let c = Vec2f::new(ax, cy);
	let d = Vec2f::new(bx, cy);

	let (top_left, top_right, bottom_left, bottom_right) = match frame.transform {
		chipty::SpriteTransform::None => (a, b, c, d),
		chipty::SpriteTransform::FlipX => (b, a, d, c),
		chipty::SpriteTransform::FlipY => (c, d, a, b),
		chipty::SpriteTransform::FlipXY => (d, c, b, a),
		chipty::SpriteTransform::Rotate90 => (c, a, d, b),
		chipty::SpriteTransform::Rotate180 => (d, c, b, a),
		chipty::SpriteTransform::Rotate270 => (b, d, a, c),
	};

	SpriteUV {
		top_left,
		top_right,
		bottom_left,
		bottom_right,
		width: frame.rect[2] as f32,
		height: frame.rect[3] as f32,
		origin: Vec2f::new(frame.origin[0] as f32, frame.origin[1] as f32),
	}
}

fn _sprite_frames(sheet: &chipty::SpriteSheet<chipty::SpriteId>, sprite: chipty::SpriteId) -> u16 {
	let Some(entry) = sheet.sprites.get(&sprite) else {
		panic!("sprite {:?} not found in sheet", sprite);
	};
	entry.len
}
