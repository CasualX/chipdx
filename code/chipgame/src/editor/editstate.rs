use super::*;

const ZOOM_STEP: f32 = 32.0;
const ZOOM_MIN: f32 = 64.0;
const ZOOM_MAX: f32 = 1024.0;

#[derive(Default)]
pub struct EditorEditState {
	pub fx: Box<FxEditState>,
	pub tool: Option<ToolState>,

	pub screen_size: Vec2<i32>,
	pub cursor_pos: Vec2<i32>,
	pub mouse_pos: Vec3<f32>,

	pub input: Input,

	history: History<String>,
}

impl EditorEditState {
	pub fn load_level(&mut self, json: &str) {
		let mut level_dto: LevelDto = serde_json::from_str(json).unwrap();
		level_dto.normalize();

		self.fx = FxEditState::new(chipcore::EditState::from_level_dto(&level_dto), &tiles::TILES);

		// Reset history to the freshly loaded level
		self.history.clear(json.to_string());
	}
	pub fn reload_level(&mut self, json: &str) {
		let mut level_dto: LevelDto = serde_json::from_str(json).unwrap();
		level_dto.normalize();

		// Reload the level but keep the camera position
		let old_cam = self.fx.camera.clone();
		self.fx = FxEditState::new(chipcore::EditState::from_level_dto(&level_dto), &tiles::TILES);
		self.fx.camera = old_cam;
	}
	pub fn save_level_dto(&self) -> chipty::LevelDto {
		self.fx.edit.to_level_dto()
	}
	pub fn save_level(&self) -> String {
		let dto = self.save_level_dto();
		serde_json::to_string(&dto).unwrap()
	}
	pub fn set_level_name(&mut self, value: String) {
		self.fx.edit.name = value;
		self.push_history();
	}
	pub fn set_author(&mut self, value: Option<String>) {
		self.fx.edit.author = value;
		self.push_history();
	}
	pub fn set_hint(&mut self, value: Option<String>) {
		self.fx.edit.hint = value;
		self.push_history();
	}
	pub fn set_required_chips(&mut self, value: i32) {
		self.fx.edit.required_chips = value.max(0);
		self.push_history();
	}
	pub fn set_time_limit(&mut self, value: i32) {
		self.fx.edit.time_limit = value.max(0);
		self.push_history();
	}
	pub fn set_screen_size(&mut self, width: i32, height: i32) {
		self.screen_size = Vec2::new(width, height);
	}
	pub fn mouse_move(&mut self, mouse_x: i32, mouse_y: i32) {
		let cam = self.fx.camera.setup(self.screen_size);
		let ray = cam.viewport_to_ray(Vec2(mouse_x, mouse_y));

		let plane = Plane3::new(Vec3::Z, 0.0);
		if let Some(hit) = ray.trace(&plane) {
			self.mouse_pos = ray.at(hit.distance);
			self.cursor_pos = self.mouse_pos.xy().map(|c| (c / 32.0).floor() as i32);
		}
		else {
			self.mouse_pos = Vec3::ZERO;
			self.cursor_pos = Vec2::ZERO;
		}
	}
	pub fn key_left(&mut self, pressed: bool) {
		self.input.key_left = pressed;
	}
	pub fn key_right(&mut self, pressed: bool) {
		self.input.key_right = pressed;
	}
	pub fn key_up(&mut self, pressed: bool) {
		self.input.key_up = pressed;
	}
	pub fn key_down(&mut self, pressed: bool) {
		self.input.key_down = pressed;
	}
	pub fn key_shift(&mut self, pressed: bool) {
		self.input.key_shift = pressed;
	}
	pub fn zoom_in(&mut self) {
		self.fx.camera.zoom_by(-ZOOM_STEP, ZOOM_MIN, ZOOM_MAX);
	}
	pub fn zoom_out(&mut self) {
		self.fx.camera.zoom_by(ZOOM_STEP, ZOOM_MIN, ZOOM_MAX);
	}
	pub fn think(&mut self) {
	}
	pub fn draw(&mut self, g: &mut shade::Graphics, resx: &fx::Resources, time: f64) {
		if self.input.key_left {
			self.fx.camera.pan_free_roam(Vec2f(-5.0, 0.0), self.fx.time);
		}
		if self.input.key_right {
			self.fx.camera.pan_free_roam(Vec2f(5.0, 0.0), self.fx.time);
		}
		if self.input.key_up {
			self.fx.camera.pan_free_roam(Vec2f(0.0, -5.0), self.fx.time);
		}
		if self.input.key_down {
			self.fx.camera.pan_free_roam(Vec2f(0.0, 5.0), self.fx.time);
		}

		if let Some(mut tool_state) = self.tool.take() {
			tool_state.think(self);
			if self.tool.is_none() {
				self.tool = Some(tool_state);
			}
		}

		render::drawbg(g, resx);
		self.fx.draw(g, resx, time);

		let cam = self.fx.camera.setup(self.screen_size);

		g.begin(&shade::BeginArgs::Immediate {
			viewport: resx.viewport,
			color: &[resx.backcolor()],
			levels: None,
			depth: Some(resx.backdepth()),
		});

		let p = self.mouse_pos; {
			let mut cv = shade::im::DrawBuilder::<render::Vertex, render::Uniform>::new();
			cv.depth_test = Some(shade::Compare::Less);
			cv.shader = Some(resx.shader.as_ref());
			cv.uniform.transform = cam.view_proj;
			cv.uniform.texture = resx.spritesheet_texture.as_ref();
			cv.uniform.shadow_tint = Vec3::dup(1.0);

			for y in 0..TERRAIN_SAMPLES.len() as i32 {
				for x in 0..2 {
					let terrain = TERRAIN_SAMPLES[y as usize][x as usize];
					let pos = Vec3::new((x - 3) as f32 * 32.0, y as f32 * 32.0, 0.0);
					render::draw_tile(&mut cv, resx, terrain, pos, self.fx.tiles);
				}
			}

			for i in 0..ENTITY_SAMPLES.len() as i32 {
				let (_, sprite) = ENTITY_SAMPLES[i as usize];
				let pos = Vec3::new(i as f32 * 32.0, -2.0 * 32.0, 0.0);
				render::draw(&mut cv, Some(&cam), resx, pos, sprite, chipty::ModelId::ReallyFlatSprite, 0, 1.0);
			}

			match &self.tool {
				Some(ToolState::Terrain(state)) => {
					render::draw_tile(&mut cv, resx, state.selected_terrain, p, self.fx.tiles);
				}
				Some(ToolState::IcePath(_state)) => {
					render::draw_tile(&mut cv, resx, Terrain::Ice, p, self.fx.tiles);
				}
				Some(ToolState::ForcePath(_state)) => {
					render::draw_tile(&mut cv, resx, Terrain::ForceRandom, p, self.fx.tiles);
				}
				_ => (),
			}
			g.clear(&shade::ClearArgs {
				depth: Some(1.0),
				..Default::default()
			});
			cv.draw(g);
		}

		{
			let mut cv = shade::im::DrawBuilder::<menu::UiVertex, menu::UiUniform>::new();
			cv.shader = Some(resx.colorshader.as_ref());
			cv.uniform.transform = cvmath::Transform2::ortho(Bounds2::vec(self.screen_size).cast());
			cv.uniform.texture = resx.spritesheet_texture.as_ref();

			struct ToVertex {
				color: [u8; 4],
			}
			impl shade::d2::ToVertex<menu::UiVertex> for ToVertex {
				fn to_vertex(&self, pos: Point2<f32>, _: usize) -> menu::UiVertex {
					menu::UiVertex { pos: pos, uv: Vec2::ZERO, color: self.color }
				}
			}
			let mut pen = shade::d2::Pen { template: ToVertex { color: [255, 255, 255, 255] } };
			for conn in &self.fx.edit.connections {
				let terrain = self.fx.edit.get_terrain(conn.src);
				pen.template.color = match terrain {
					chipty::Terrain::GreenButton => [0, 255, 0, 255],
					chipty::Terrain::RedButton => [255, 0, 0, 255],
					chipty::Terrain::BrownButton => [128, 128, 0, 255],
					chipty::Terrain::BlueButton => [0, 0, 255, 255],
					chipty::Terrain::Teleport => [0, 255, 255, 255],
					_ => [255, 255, 255, 255], // Default color
				};
				let src = cam.world_to_viewport(conn.src.map(|c| c as f32 * 32.0 + 16.0).vec3(0.0)).unwrap();
				let dest = cam.world_to_viewport(conn.dest.map(|c| c as f32 * 32.0 + 16.0).vec3(0.0)).unwrap();
				cv.draw_arrow(&pen, src, dest, 12.0);
			}

			let pen = shade::d2::Pen { template: ToVertex { color: [255, 0, 0, 255] } };
			for (_, ent) in self.fx.edit.entities_in_order() {
				let pos = cam.world_to_viewport(ent.pos.map(|c| c as f32 * 32.0 + 16.0).vec3(0.0)).unwrap();
				if let Some(face_dir) = ent.face_dir {
					cv.draw_arrow(&pen, pos, pos + face_dir.to_vec().map(|c| c as f32 * 20.0), 4.0);
				}
				cv.draw_line_rect(&pen, &Bounds2::new(pos - Vec2::dup(4.0), pos + Vec2::dup(4.0)));
			}
			cv.draw(g);
		}

		if let Some(ToolState::EntOrder(_state)) = &self.tool {
			fxeditstate::draw_entity_order(&self.fx, g, resx, &cam);
		}

		if let Some(tool_state) = &self.tool {
			let text = fmtools::format!(
				"Tool: "{tool_state.name()}"\n"
				if let Some(display) = tool_state.display() {
					{tool_state.name()}": "{display}"\n"
				}
				"Cursor: ("{self.cursor_pos.x}", "{self.cursor_pos.y}")\n"
			);

			menu::draw_overlay(g, resx, shade::d2::TextAlign::BottomRight, &text);
		}

		g.end();
	}

	fn push_history(&mut self) {
		let snapshot = self.save_level();
		self.history.push_if(snapshot);
	}
	pub fn undo(&mut self) {
		if let Some(prev) = self.history.undo().cloned() {
			self.reload_level(&prev);
			eprintln!("Undo");
		}
	}
	pub fn redo(&mut self) {
		if let Some(next) = self.history.redo().cloned() {
			self.reload_level(&next);
			eprintln!("Redo");
		}
	}

	pub fn tool_terrain(&mut self, pressed: bool) {
		if pressed {
			self.tool = Some(ToolState::Terrain(Default::default()));
		}
	}
	pub fn tool_entity(&mut self, pressed: bool) {
		if pressed {
			self.tool = Some(ToolState::Entity(Default::default()));
		}
	}
	pub fn tool_connection(&mut self, pressed: bool) {
		if pressed {
			self.tool = Some(ToolState::Connection(Default::default()));
		}
	}
	pub fn tool_icepath(&mut self, pressed: bool) {
		if pressed {
			self.tool = Some(ToolState::IcePath(Default::default()));
		}
	}
	pub fn tool_forcepath(&mut self, pressed: bool) {
		if pressed {
			self.tool = Some(ToolState::ForcePath(Default::default()));
		}
	}
	pub fn tool_entorder(&mut self, pressed: bool) {
		if pressed {
			self.tool = Some(ToolState::EntOrder(Default::default()));
		}
	}

	/// Resizes the playfield.
	///
	/// Positive values = expand, Negative values = crop.
	pub fn resize(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
		let terrain = if let Some(ToolState::Terrain(state)) = &self.tool {
			state.selected_terrain
		}
		else {
			chipty::Terrain::Floor
		};
		if self.fx.resize(left, top, right, bottom, terrain) {
			self.push_history();
		}
	}

	pub fn left_click(&mut self, pressed: bool) {
		if pressed && (self.cursor_pos.x < 0 || self.cursor_pos.y < 0) {
			self.sample();
			return;
		}

		if let Some(mut tool_state) = self.tool.take() {
			tool_state.left_click(self, pressed);
			if self.tool.is_none() {
				self.tool = Some(tool_state);
			}
		}

		if !pressed {
			self.push_history();
		}

		self.input.left_click = pressed;
	}
	pub fn right_click(&mut self, pressed: bool) {
		if let Some(mut tool_state) = self.tool.take() {
			tool_state.right_click(self, pressed);
			if self.tool.is_none() {
				self.tool = Some(tool_state);
			}
		}

		if !pressed {
			self.push_history();
		}

		self.input.right_click = pressed;
	}
	pub fn delete(&mut self, pressed: bool) {
		if let Some(mut tool_state) = self.tool.take() {
			tool_state.delete(self, pressed);
			if self.tool.is_none() {
				self.tool = Some(tool_state);
			}
		}

		if !pressed {
			self.push_history();
		}
	}

	pub fn sample(&mut self) {
		let s = self;
		let cursor_pos = s.cursor_pos;

		// Sample from the terrain samples
		if cursor_pos.x < 0 {
			if cursor_pos.x == -3 && cursor_pos.y >= 0 && cursor_pos.y < TERRAIN_SAMPLES.len() as i32 {
				let selected_terrain = TERRAIN_SAMPLES[cursor_pos.y as usize][0];
				s.tool = Some(ToolState::Terrain(TerrainToolState { selected_terrain }));
			}
			if cursor_pos.x == -2 && cursor_pos.y >= 0 && cursor_pos.y < TERRAIN_SAMPLES.len() as i32 {
				let selected_terrain = TERRAIN_SAMPLES[cursor_pos.y as usize][1];
				s.tool = Some(ToolState::Terrain(TerrainToolState { selected_terrain }));
			}
		}
		// Sample from the entity samples
		else if cursor_pos.y < 0 {
			if cursor_pos.y == -2 && cursor_pos.x >= 0 && cursor_pos.x < ENTITY_SAMPLES.len() as i32 {
				let (kind, _) = ENTITY_SAMPLES[cursor_pos.x as usize];
				let selected_ent = None;
				let selected_args = Some(EntityArgs { kind, pos: cursor_pos, face_dir: None });
				s.tool = Some(ToolState::Entity(EntityToolState { selected_ent, selected_args }));
			}
		}
		else {
			// Sample from the existing entities
			let ehandle = s.fx.edit.entity_at(cursor_pos);
			if let Some(ehandle) = ehandle {
				if let Some(ent) = s.fx.edit.entity(ehandle) {
					let selected_ent = Some(ehandle);
					let selected_args = Some(*ent);
					s.tool = Some(ToolState::Entity(EntityToolState { selected_ent, selected_args }));
					println!("Selected: {:?} at {}", ent.kind, ent.pos);
				}
			}
			// Sample from the terrain
			else {
				let selected_terrain = s.fx.edit.get_terrain(cursor_pos);
				s.tool = Some(ToolState::Terrain(TerrainToolState { selected_terrain }));
			}
		}

		if let Some(tool_state) = &s.tool {
			println!("Tool: {}", tool_state.name());
		}
	}
}
