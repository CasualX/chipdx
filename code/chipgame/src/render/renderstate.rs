use super::*;
#[derive(Clone, Default)]
pub struct RenderField {
	pub width: i32,
	pub height: i32,
	pub terrain: Vec<chipty::Terrain>,
}

impl RenderField {
	pub fn get_terrain(&self, pos: Vec2i) -> chipty::Terrain {
		let Vec2i { x, y } = pos;
		if x < 0 || y < 0 || x >= self.width || y >= self.height {
			return chipty::Terrain::Blank;
		}
		let index = (y * self.width + x) as usize;
		self.terrain.get(index).cloned().unwrap_or(chipty::Terrain::Blank)
	}
	pub fn set_terrain(&mut self, pos: Vec2i, terrain: chipty::Terrain) {
		let Vec2i { x, y } = pos;
		if x < 0 || y < 0 || x >= self.width || y >= self.height {
			return;
		}
		let index = (y * self.width + x) as usize;
		if let Some(ptr) = self.terrain.get_mut(index) {
			*ptr = terrain;
		}
	}
}

pub struct UpdateCtx {
	pub time: f64,
	pub dt: f64,
}

#[derive(Default)]
pub struct RenderState {
	pub objects: ObjectMap,
	pub field: RenderField,
	pub effects: Vec<Effect>,
	pub tiles: &'static [TileGfx],
	pub shadow_map: Option<Box<dyn shade::Texture2D>>,
	pub light_matrix: Mat4f,
}

impl Clone for RenderState {
	fn clone(&self) -> Self {
		Self {
			objects: self.objects.clone(),
			field: self.field.clone(),
			effects: self.effects.clone(),
			tiles: self.tiles,
			shadow_map: None,
			light_matrix: self.light_matrix,
		}
	}
}

impl RenderState {
	pub fn shadow_map(&self) -> &dyn shade::Texture2D {
		&**self.shadow_map.as_ref().expect("shadow map texture has not been created")
	}

	pub fn clear(&mut self) {
		self.objects.clear();
		self.field.width = 0;
		self.field.height = 0;
		self.field.terrain.clear();
		self.effects.clear();
	}
	pub fn update(&mut self, ctx: &UpdateCtx) {
		self.objects.retain(|_, obj| obj.update(ctx));
		self.effects.retain(|efx| ctx.time < efx.start + 1.0);
	}
	pub fn draw(&self, g: &mut shade::Graphics, resx: &Resources, camera: &shade::d3::Camera, time: f64, vision_clip: Option<(Vec2f, f32)>) {
		g.begin(&shade::BeginArgs::Immediate {
			viewport: resx.viewport,
			color: &[resx.backcolor()],
			levels: None,
			depth: Some(resx.backdepth()),
		});

		self.draw_field(g, resx, camera, time, false, vision_clip);
		self.draw_effects(g, resx, camera, time, vision_clip);

		g.end();
	}
	pub fn draw_field(&self, g: &mut shade::Graphics, resx: &Resources, camera: &shade::d3::Camera, time: f64, shadow: bool, vision_clip: Option<(Vec2f, f32)>) {
		let mut cv = shade::im::DrawBuilder::<render::Vertex, render::Uniform>::new();
		cv.depth_test = Some(shade::Compare::LessEqual);
		cv.cull_mode = Some(shade::CullMode::CW);
		cv.shader = Some(if shadow { resx.shader_shadowmap.as_ref() } else { resx.shader.as_ref() });
		cv.uniform.transform = camera.view_proj;
		cv.uniform.texture = resx.spritesheet_texture.as_ref();
		cv.uniform.shadow_map = self.shadow_map();
		cv.uniform.light_matrix = self.light_matrix;
		if !shadow {
			if let Some((center, half_extent)) = vision_clip {
				cv.uniform.vision_center = center;
				cv.uniform.vision_half_extent = half_extent;
			}
		}
		render::field(&mut cv, camera, self, resx, time);
		cv.draw(g);
	}
	pub fn draw_effects(&self, g: &mut shade::Graphics, resx: &Resources, camera: &shade::d3::Camera, time: f64, vision_clip: Option<(Vec2f, f32)>) {
		let mut cv = shade::im::DrawBuilder::<Vertex, Uniform>::new();
		cv.depth_test = Some(shade::Compare::Always);
		// cv.cull_mode = Some(shade::CullMode::CW);

		cv.shader = Some(resx.shader.as_ref());
		cv.uniform.transform = camera.view_proj;
		cv.uniform.texture = resx.effects.as_ref();
		if let Some((center, half_extent)) = vision_clip {
			cv.uniform.vision_center = center;
			cv.uniform.vision_half_extent = half_extent;
		}

		for efx in &self.effects {
			efx.draw(&mut cv, time);
		}
		cv.draw(g);
	}
}
