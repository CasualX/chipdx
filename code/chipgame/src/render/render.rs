use super::*;

// Vertex definition

#[derive(Copy, Clone, Debug, Default, dataview::Pod)]
#[repr(C)]
pub struct Vertex {
	pub pos: Vec3<f32>,
	pub uv: Vec2<f32>,
	pub color: [u8; 4],
}

unsafe impl shade::TVertex for Vertex {
	const LAYOUT: &'static shade::VertexLayout = &shade::VertexLayout {
		size: std::mem::size_of::<Vertex>() as u16,
		alignment: std::mem::align_of::<Vertex>() as u16,
		attributes: &[
			shade::VertexAttribute {
				name: "a_pos",
				format: shade::VertexAttributeFormat::F32v3,
				offset: dataview::offset_of!(Vertex.pos) as u16,
			},
			shade::VertexAttribute {
				name: "a_texcoord",
				format: shade::VertexAttributeFormat::F32v2,
				offset: dataview::offset_of!(Vertex.uv) as u16,
			},
			shade::VertexAttribute {
				name: "a_color",
				format: shade::VertexAttributeFormat::U8Normv4,
				offset: dataview::offset_of!(Vertex.color) as u16,
			},
		],
	};
}

#[derive(Clone, Debug, PartialEq)]
pub struct Uniform {
	pub transform: Mat4f,
	pub texture: shade::Texture2D,
	pub greyscale: f32,

	pub shadow_map: shade::Texture2D,
	pub light_matrix: Mat4f,
	pub shadow_bias: f32,
	pub shadow_tint: Vec3f,
}

impl Default for Uniform {
	fn default() -> Uniform {
		Uniform {
			transform: Mat4::IDENTITY,
			texture: shade::Texture2D::INVALID,
			greyscale: 0.0,

			shadow_map: shade::Texture2D::INVALID,
			light_matrix: Mat4::IDENTITY,
			shadow_bias: 0.002,
			shadow_tint: Vec3(0.75, 0.80, 0.90),
		}
	}
}

impl shade::UniformVisitor for Uniform {
	fn visit(&self, set: &mut dyn shade::UniformSetter) {
		set.value("u_transform", &self.transform);
		set.value("u_tex", &self.texture);
		set.value("u_greyscale", &self.greyscale);
		set.value("u_shadow_map", &self.shadow_map);
		set.value("u_light_matrix", &self.light_matrix);
		set.value("u_shadow_bias", &self.shadow_bias);
		set.value("u_shadow_tint", &self.shadow_tint);
	}
}

const T_S: f32 = 32.0; // TILE SIZE

pub struct ModelData {
	pub vertices: &'static [Vertex],
	pub indices: &'static [u32],
}

impl ModelData {
	pub const FLOOR: ModelData = ModelData {
		vertices: &[
			Vertex { pos: Vec3(0.0, 0.0, 0.0), uv: Vec2(0.0, 0.0), color: [255, 255, 255, 255] },
			Vertex { pos: Vec3(0.0, T_S, 0.0), uv: Vec2(0.0, 1.0), color: [255, 255, 255, 255] },
			Vertex { pos: Vec3(T_S, T_S, 0.0), uv: Vec2(1.0, 1.0), color: [255, 255, 255, 255] },
			Vertex { pos: Vec3(T_S, 0.0, 0.0), uv: Vec2(1.0, 0.0), color: [255, 255, 255, 255] },
		],
		indices: &[0, 1, 2, 0, 2, 3],
	};

	pub const WALL: ModelData = {
		const S: f32 = 4.0;
		const H: f32 = 20.0;
		ModelData {
			vertices: &[
				Vertex { pos: Vec3(0.0, 0.0, 0.0), uv: Vec2(0.0, 0.0), color: [255, 255, 255, 255] },
				Vertex { pos: Vec3(0.0, T_S, 0.0), uv: Vec2(0.0, 1.0), color: [255, 255, 255, 255] },
				Vertex { pos: Vec3(T_S, T_S, 0.0), uv: Vec2(1.0, 1.0), color: [255, 255, 255, 255] },
				Vertex { pos: Vec3(T_S, 0.0, 0.0), uv: Vec2(1.0, 0.0), color: [255, 255, 255, 255] },
				Vertex { pos: Vec3(      S,       S,   H), uv: Vec2(0.0, 0.1), color: [255, 255, 255, 255] },
				Vertex { pos: Vec3(      S, T_S - S,   H), uv: Vec2(0.0, 0.9), color: [255, 255, 255, 255] },
				Vertex { pos: Vec3(T_S - S, T_S - S,   H), uv: Vec2(0.9, 0.9), color: [255, 255, 255, 255] },
				Vertex { pos: Vec3(T_S - S,       S,   H), uv: Vec2(0.9, 0.1), color: [255, 255, 255, 255] },
			],
			indices: &[
				0, 1, 4, 4, 1, 5,
				1, 2, 5, 5, 2, 6,
				2, 3, 6, 6, 3, 7,
				3, 0, 7, 7, 0, 4,
				4, 6, 7, 4, 5, 6,
			],
		}
	};

	pub const SPRITE: ModelData = ModelData {
		vertices: &[
			Vertex { pos: Vec3(0.0, 0.0, 20.0), uv: Vec2(0.0, 0.0), color: [255, 255, 255, 255] },
			Vertex { pos: Vec3(0.0, T_S, 20.0), uv: Vec2(0.0, 1.0), color: [255, 255, 255, 255] },
			Vertex { pos: Vec3(T_S, T_S, 20.0), uv: Vec2(1.0, 1.0), color: [255, 255, 255, 255] },
			Vertex { pos: Vec3(T_S, 0.0, 20.0), uv: Vec2(1.0, 0.0), color: [255, 255, 255, 255] },
		],
		indices: &[0, 1, 2, 0, 2, 3],
	};

	pub const SPRITE_SHADOW: ModelData = ModelData {
		vertices: &[
			Vertex { pos: Vec3(0.0, 0.0, 0.5), uv: Vec2(0.0, 0.0), color: [0, 0, 0, 128] },
			Vertex { pos: Vec3(0.0, T_S, 0.5), uv: Vec2(0.0, 1.0), color: [0, 0, 0, 128] },
			Vertex { pos: Vec3(T_S, T_S, 0.5), uv: Vec2(1.0, 1.0), color: [0, 0, 0, 128] },
			Vertex { pos: Vec3(T_S, 0.0, 0.5), uv: Vec2(1.0, 0.0), color: [0, 0, 0, 128] },
		],
		indices: &[0, 1, 2, 0, 2, 3],
	};
}

fn draw_floor(cv: &mut shade::im::DrawBuilder<Vertex, Uniform>, spr: &SpriteUV, pos: Vec3<f32>, z1: f32, z2: f32, alpha: f32) {
	let mut p = cv.begin(shade::PrimType::Triangles, 4, 2);

	p.add_indices_quad();

	let x = pos.x - spr.origin.x;
	let y = pos.y - spr.origin.y;
	let z1 = z1 + pos.z;
	let z2 = z2 + pos.z;

	let color = [255, 255, 255, (alpha * 255.0) as u8];

	p.add_vertex(Vertex {
		pos: Vec3(x, y, z2),
		uv: spr.top_left,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x, y + spr.height, z1),
		uv: spr.bottom_left,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width, y + spr.height, z1),
		uv: spr.bottom_right,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width, y, z2),
		uv: spr.top_right,
		color,
	});
}

fn draw_wall(cv: &mut shade::im::DrawBuilder<Vertex, Uniform>, spr: &SpriteUV, pos: Vec3<f32>, base: f32, alpha: f32) {
	let mut p = cv.begin(shade::PrimType::Triangles, 8, 10);

	p.add_indices(&[
		0, 1, 4, 4, 1, 5,
		1, 2, 5, 5, 2, 6,
		2, 3, 6, 6, 3, 7,
		3, 0, 7, 7, 0, 4,
		4, 6, 7, 4, 5, 6,
	]);

	let x = pos.x;
	let y = pos.y;
	let z = pos.z;

	let s = 4.0 + base;//if matches!(sprite, data::Sprite::Wall) { 0.0 } else { 4.0 };
	let t = 4.0;
	let tall = 20.0; //if block.is_door() { 15.0 } else { 20.0 };

	let color = [255, 255, 255, (alpha * 255.0) as u8];

	p.add_vertex(Vertex {
		pos: Vec3(x + base, y + base, z),
		uv: spr.top_left,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + base, y + spr.height - base, z),
		uv: spr.bottom_left,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width - base, y + spr.height - base, z),
		uv: spr.bottom_right,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width - base, y + base, z),
		uv: spr.top_right,
		color,
	});

	p.add_vertex(Vertex {
		pos: Vec3(x + s, y + s, z + tall),
		uv: spr.top_left + Vec2(t, t),
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + s, y + spr.height - s, z + tall),
		uv: spr.bottom_left + Vec2(t, -t),
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width - s, y + spr.height - s, z + tall),
		uv: spr.bottom_right + Vec2(-t, -t),
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width - s, y + s, z + tall),
		uv: spr.top_right + Vec2(-t, t),
		color,
	});
}

fn draw_portal(cv: &mut shade::im::DrawBuilder<Vertex, Uniform>, spr: &SpriteUV, pos: Vec3<f32>) {
	let mut p = cv.begin(shade::PrimType::Triangles, 5, 4);

	p.add_indices(&[0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 1]);

	let x = pos.x;
	let y = pos.y;
	let z = pos.z;

	let cx = x + spr.width * 0.5;
	let cy = y + spr.height * 0.5;

	let color = [255, 255, 255, 255];

	p.add_vertex(Vertex {
		pos: Vec3(cx, cy, z - 10.0),
		uv: (spr.top_left + spr.bottom_right) * 0.5,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x, y, z),
		uv: spr.top_left,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x, y + spr.height, z),
		uv: spr.bottom_left,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width, y + spr.height, z),
		uv: spr.bottom_right,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width, y, z),
		uv: spr.top_right,
		color,
	});
}

fn draw_tank(cv: &mut shade::im::DrawBuilder<Vertex, Uniform>, spr: &SpriteUV, pos: Vec3<f32>, face_dir: Option<chipty::Compass>, alpha: f32) {
	let mut p = cv.begin(shade::PrimType::Triangles, 8, 10);

	p.add_indices(&[
		0, 1, 4, 4, 1, 5,
		1, 2, 5, 5, 2, 6,
		2, 3, 6, 6, 3, 7,
		3, 0, 7, 7, 0, 4,
		4, 6, 7, 4, 5, 6,
	]);

	let x = pos.x;
	let y = pos.y;
	let z = pos.z;
	let base = 0.0;

	let s = 8.0 + base;//if matches!(sprite, data::Sprite::Wall) { 0.0 } else { 4.0 };
	let t = 8.0;
	let tall = 8.0; //if block.is_door() { 15.0 } else { 20.0 };

	let color = [255, 255, 255, (alpha * 255.0) as u8];

	_ = face_dir;

	p.add_vertex(Vertex {
		pos: Vec3(x + base, y + base, z),
		uv: spr.top_left,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + base, y + spr.height - base, z),
		uv: spr.bottom_left,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width - base, y + spr.height - base, z),
		uv: spr.bottom_right,
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width - base, y + base, z),
		uv: spr.top_right,
		color,
	});

	p.add_vertex(Vertex {
		pos: Vec3(x + s, y + s, z + tall),
		uv: spr.top_left + Vec2(t, t),
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + s, y + spr.height - s, z + tall),
		uv: spr.bottom_left + Vec2(t, -t),
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width - s, y + spr.height - s, z + tall),
		uv: spr.bottom_right + Vec2(-t, -t),
		color,
	});
	p.add_vertex(Vertex {
		pos: Vec3(x + spr.width - s, y + s, z + tall),
		uv: spr.top_right + Vec2(-t, t),
		color,
	});
}

pub fn draw(
	cv: &mut shade::im::DrawBuilder<Vertex, Uniform>,
	camera: Option<&shade::d3::Camera>,
	resx: &Resources,
	pos: Vec3<f32>,
	sprite: chipty::SpriteId,
	model: chipty::ModelId,
	frame: u16,
	alpha: f32,
) {
	if alpha <= 0.0 || matches!(sprite, chipty::SpriteId::Blank) || matches!(model, chipty::ModelId::Empty) {
		return;
	}
	if let Some(camera) = camera {
		let bounds = Bounds3(pos, pos + Vec3(32.0, 32.0, 32.0));
		if !camera.is_visible(&bounds, None) {
			return;
		}
	}

	let spr = sprite_uv(&resx.spritesheet_meta, sprite, frame);
	let face_dir = sprite.face_dir();

	match model {
		chipty::ModelId::Empty => (),
		chipty::ModelId::Floor => draw_floor(cv, &spr, pos, 0.0, 0.0, alpha),
		chipty::ModelId::Wall => draw_wall(cv, &spr, pos, 0.0, alpha),
		chipty::ModelId::ToggleWall => draw_wall(cv, &spr, pos, 2.0, alpha),
		chipty::ModelId::Sprite => draw_floor(cv, &spr, pos, 0.0, 20.0, alpha),
		chipty::ModelId::EndPortal => draw_portal(cv, &spr, pos),
		chipty::ModelId::FlatSprite => draw_floor(cv, &spr, pos, 3.0, 12.0, alpha),
		chipty::ModelId::ReallyFlatSprite => draw_floor(cv, &spr, pos, 6.0, 10.0, alpha),
		chipty::ModelId::FloorSprite => draw_floor(cv, &spr, pos, 1.0, 1.0, alpha),
		chipty::ModelId::Tank => draw_tank(cv, &spr, pos, face_dir, alpha),
		chipty::ModelId::Glider => unimplemented!(),//draw_glider(cv, &spr, pos, face_dir, alpha),
	}
}

pub fn draw_tile(cv: &mut shade::im::DrawBuilder::<render::Vertex, render::Uniform>, resx: &Resources, terrain: chipty::Terrain, pos: Vec3<f32>, tiles: &[TileGfx]) {
	let tile = tiles[terrain as usize];
	draw(cv, None, resx, pos, tile.sprite, tile.model, 0, 1.0);
}

pub fn field(cv: &mut shade::im::DrawBuilder::<render::Vertex, render::Uniform>, camera: &shade::d3::Camera, fx: &RenderState, resx: &Resources, time: f64) {
	// Render the level geometry
	cv.blend_mode = shade::BlendMode::Solid;
	let frame = (time * 8.0) as i32 as u16;
	let field = &fx.field;
	for y in 0..field.height {
		for x in 0..field.width {
			let tile = field.get_terrain(Vec2(x, y));
			let tile = fx.tiles[tile as usize];
			if tile.sprite == chipty::SpriteId::Blank || tile.model == chipty::ModelId::Empty {
				continue;
			}
			let (sprite, model) = (tile.sprite, tile.model);
			draw(cv, Some(camera), resx, Vec2(x, y).map(|c| c as f32 * 32.0).vec3(0.0), sprite, model, frame, 1.0);
		}
	}

	// Collect and sort the objects by Y position
	// Some sprites are slightly offset in Y, so round to the nearest integer tile
	let mut sorted_objects: Vec<_> = fx.objects.values().map(|obj| &obj.data).collect();
	sorted_objects.sort_unstable_by_key(|obj| ((obj.pos.y / 32.0).round() as i32, obj.model, obj.pos.z as i32, obj.pos.x as i32));

	// Render the objects
	cv.blend_mode = shade::BlendMode::Alpha;
	for obj in &sorted_objects {
		// Grayscale the template entities
		cv.uniform.greyscale = if obj.greyscale { 1.0 } else { 0.0 };
		// Configure depth testing
		cv.depth_test = if obj.depth_test { Some(shade::Compare::LessEqual) } else { None };
		// Draw the object
		draw(cv, Some(camera), resx, obj.pos, obj.sprite, obj.model, obj.frame, obj.alpha);
	}
}
