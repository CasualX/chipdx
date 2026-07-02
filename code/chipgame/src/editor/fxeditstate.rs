use super::*;

pub struct FxEditState {
	pub edit: chipcore::EditState,
	pub camera: fx::PlayCamera,
	pub time: f64,
	pub dt: f64,
	pub random: fx::Random,
	pub tiles: render::TileGfxFn,
}

impl FxEditState {
	pub fn new(edit: chipcore::EditState, tiles: render::TileGfxFn) -> Box<FxEditState> {
		let mut fx = Box::new(FxEditState {
			edit,
			camera: fx::PlayCamera::default(),
			time: 0.0,
			dt: 0.0,
			random: fx::Random::default(),
			tiles,
		});
		fx.init_camera();
		fx
	}

	pub fn set_terrain(&mut self, pos: Vec2i, terrain: chipty::Terrain) {
		self.edit.set_terrain(pos, terrain);
		if matches!(terrain, chipty::Terrain::DirtBlock) {
			let ids: Vec<_> = self.edit.entities_in_order()
				.filter_map(|(id, args)| {
					if args.pos == pos && matches!(args.kind, chipty::EntityKind::Block) {
						Some(id)
					}
					else {
						None
					}
				})
				.collect();
			for id in ids {
				self.edit.remove_entity(id);
			}
		}
	}

	pub fn resize(&mut self, left: i32, top: i32, right: i32, bottom: i32, fill: chipty::Terrain) -> bool {
		let resized = self.edit.resize(left, top, right, bottom, fill);
		if resized {
			self.update_camera_bounds();
		}
		resized
	}

	pub fn create_entity(&mut self, args: chipty::EntityArgs) -> chipcore::EditEntityId {
		self.edit.create_entity(args)
	}

	pub fn remove_entity(&mut self, id: chipcore::EditEntityId) -> Option<chipty::EntityArgs> {
		self.edit.remove_entity(id)
	}

	pub fn move_entity(&mut self, id: chipcore::EditEntityId, pos: Vec2i) -> bool {
		self.edit.move_entity(id, pos)
	}

	pub fn rotate_entity(&mut self, id: chipcore::EditEntityId) -> bool {
		self.edit.rotate_entity(id)
	}

	pub fn swap_entity_order(&mut self, a: chipcore::EditEntityId, b: chipcore::EditEntityId) -> bool {
		self.edit.swap_entity_order(a, b)
	}

	pub fn brush_apply(&mut self, pos: Vec2i, brush: &chipty::LevelBrush) {
		self.edit.brush_apply(pos, brush);
	}

	pub fn toggle_connection(&mut self, conn: chipty::FieldConn) {
		self.edit.toggle_connection(conn);
	}

	pub fn draw(&mut self, g: &mut shade::Graphics, resx: &fx::Resources, time: f64) {
		self.dt = time - self.time;
		self.time = time;

		self.camera.animate_blend();
		self.camera.animate_position(self.time, self.dt, resx.viewport.size());
		self.camera.shake.update(self.dt, &mut self.random);

		let camera = self.camera.setup(resx.viewport.size());

		g.begin(&shade::BeginArgs::Immediate {
			viewport: resx.viewport,
			color: &[resx.backcolor()],
			levels: None,
			depth: Some(resx.backdepth()),
		});
		self.draw_field(g, resx, &camera);
		g.end();
	}

	fn init_camera(&mut self) {
		self.update_camera_bounds();
		let controller = fx::PositionController { target: Vec2f(26.0 * 16.0, 20.0 * 16.0) };
		self.camera.set_controller(fx::Controller::FreeRoam(controller), self.time);
		self.camera.set_perspective(false);
		self.camera.set_zoom_mode(chipty::ZoomMode::Editor, false, self.time);
	}

	fn update_camera_bounds(&mut self) {
		self.camera.bounds.mins = Vec2::ZERO;
		self.camera.bounds.maxs = Vec2(self.edit.width as f32 * 32.0, self.edit.height as f32 * 32.0);
	}

	fn draw_field(&self, g: &mut shade::Graphics, resx: &fx::Resources, camera: &shade::d3::Camera) {
		let mut cv = shade::im::DrawBuilder::<render::Vertex, render::Uniform>::new();
		cv.depth_test = Some(shade::Compare::LessEqual);
		cv.cull_mode = Some(shade::CullMode::CW);
		cv.shader = Some(resx.shader.as_ref());
		cv.uniform.transform = camera.view_proj;
		cv.uniform.texture = resx.sprites_texture.as_ref();
		cv.uniform.shadow_tint = Vec3::dup(1.0);

		self.draw_tiles(&mut cv, resx, camera);
		self.draw_terrain_overlays(&mut cv, resx, camera);
		cv.draw(g);

		g.clear(&shade::ClearArgs {
			depth: Some(1.0),
			..Default::default()
		});

		let mut cv = shade::im::DrawBuilder::<render::Vertex, render::Uniform>::new();
		cv.depth_test = Some(shade::Compare::LessEqual);
		cv.cull_mode = Some(shade::CullMode::CW);
		cv.shader = Some(resx.shader.as_ref());
		cv.uniform.transform = camera.view_proj;
		cv.uniform.texture = resx.sprites_texture.as_ref();
		cv.uniform.shadow_tint = Vec3::dup(1.0);

		self.draw_entities(&mut cv, resx, camera);
		cv.draw(g);
	}

	fn draw_tiles(&self, cv: &mut shade::im::DrawBuilder<render::Vertex, render::Uniform>, resx: &fx::Resources, camera: &shade::d3::Camera) {
		cv.blend_mode = shade::BlendMode::Solid;
		for y in 0..self.edit.height {
			for x in 0..self.edit.width {
				let pos = Vec2(x, y);
				let terrain = self.edit.get_terrain(pos);
				let tile = (self.tiles)(terrain);
				let frame = terrain_frame(self.time, terrain);
				render::draw(cv, Some(camera), resx, pos.map(|c| c as f32 * 32.0).vec3(0.0), tile.sprite, tile.model, frame, 1.0);
			}
		}
	}

	fn draw_terrain_overlays(&self, cv: &mut shade::im::DrawBuilder<render::Vertex, render::Uniform>, resx: &fx::Resources, camera: &shade::d3::Camera) {
		cv.blend_mode = shade::BlendMode::Alpha;
		for y in 0..self.edit.height {
			for x in 0..self.edit.width {
				let pos = Vec2(x, y);
				let world_pos = Vec3::new(x as f32 * 32.0, y as f32 * 32.0, 0.0);
				match self.edit.get_terrain(pos) {
					chipty::Terrain::Fire => render::draw(cv, Some(camera), resx, world_pos - Vec3(0.0, 2.0, 0.0), chipty::SpriteId::FireA, chipty::ModelId::Sprite, anim_loop_frame(self.time, 8.0), 1.0),
					chipty::Terrain::WaterHazard => render::draw(cv, Some(camera), resx, world_pos, chipty::SpriteId::WaterHazard, chipty::ModelId::Sprite, 0, 1.0),
					_ => {}
				}
			}
		}
	}

	fn draw_entities(&self, cv: &mut shade::im::DrawBuilder<render::Vertex, render::Uniform>, resx: &fx::Resources, camera: &shade::d3::Camera) {
		let mut entities: Vec<_> = self.edit.entities_in_order()
			.map(|(_, args)| {
				let pos = entity_pos(args);
				(pos, *args)
			})
			.collect();
		entities.sort_unstable_by_key(|(pos, args)| ((pos.y / 32.0).round() as i32, model_for_entity_kind(args.kind), pos.z as i32, pos.x as i32));

		cv.blend_mode = shade::BlendMode::Alpha;
		for (pos, args) in entities {
			let sprite = sprite_for_entity_args(&self.edit, &args);
			let model = model_for_entity_kind(args.kind);
			let frame = entity_frame(self.time, args.kind);
			render::draw(cv, Some(camera), resx, pos, sprite, model, frame, 1.0);
		}
	}
}

pub fn draw_entity_order(fx: &FxEditState, g: &mut shade::Graphics, resx: &fx::Resources, camera: &shade::d3::Camera) {
	let mut tbuf = shade::d2::TextBuffer::new();
	tbuf.shader = Some(&*resx.font.shader);
	tbuf.blend_mode = shade::BlendMode::Alpha;
	tbuf.uniform.texture = &*resx.font.texture;
	tbuf.uniform.transform = cvmath::Transform2::ortho(resx.viewport.cast());

	let size = resx.viewport.height() as f32 / 32.0;
	let scribe = shade::d2::Scribe {
		font_size: size,
		line_height: size,
		..Default::default()
	};
	for (index, (_, args)) in fx.edit.entities_in_order().enumerate() {
		let Some(pos) = camera.world_to_viewport(entity_pos(args) + Vec3(16.0, 16.0, 0.0)) else { continue };
		tbuf.text_box(&resx.font, &scribe, &cvmath::Bounds2::point(pos, Vec2::ZERO), shade::d2::TextAlign::MiddleCenter, &format!("{}", index));
	}

	tbuf.draw(g);
}

fn anim_loop_frame(time: f64, frame_rate: f32) -> u16 {
	(time * frame_rate as f64) as u16
}

fn anim_seq_frame(time: f64, frame_rate: f32, frame_count: u16) -> u16 {
	if frame_count == 0 {
		0
	}
	else {
		anim_loop_frame(time, frame_rate) % frame_count
	}
}

fn terrain_frame(time: f64, terrain: chipty::Terrain) -> u16 {
	match terrain {
		chipty::Terrain::Exit |
		chipty::Terrain::Water |
		chipty::Terrain::Fire |
		chipty::Terrain::ToggleFloor |
		chipty::Terrain::ToggleWall |
		chipty::Terrain::ForceN |
		chipty::Terrain::ForceS |
		chipty::Terrain::ForceE |
		chipty::Terrain::ForceW |
		chipty::Terrain::ForceRandom => anim_loop_frame(time, 8.0),
		_ => 0,
	}
}

fn entity_frame(time: f64, kind: chipty::EntityKind) -> u16 {
	match kind {
		chipty::EntityKind::Bomb => anim_loop_frame(time, 16.0),
		chipty::EntityKind::FireBall |
		chipty::EntityKind::Bug |
		chipty::EntityKind::Glider |
		chipty::EntityKind::Walker |
		chipty::EntityKind::Teeth |
		chipty::EntityKind::Blob |
		chipty::EntityKind::Paramecium => anim_seq_frame(time, 16.0, 4),
		_ => 0,
	}
}

fn entity_pos(args: &chipty::EntityArgs) -> Vec3f {
	let base_z = match args.kind {
		chipty::EntityKind::Socket => 2.0,
		chipty::EntityKind::Thief => 1.0,
		_ => 0.0,
	};
	Vec3::new(args.pos.x as f32 * 32.0, args.pos.y as f32 * 32.0, base_z)
}

fn model_for_entity_kind(kind: chipty::EntityKind) -> chipty::ModelId {
	match kind {
		chipty::EntityKind::Block => chipty::ModelId::Wall,
		chipty::EntityKind::IceBlock => chipty::ModelId::Wall,
		chipty::EntityKind::Tank => chipty::ModelId::Tank,
		chipty::EntityKind::Bug => chipty::ModelId::FlatSprite,
		chipty::EntityKind::Blob => chipty::ModelId::ReallyFlatSprite,
		chipty::EntityKind::Paramecium => chipty::ModelId::ReallyFlatSprite,
		_ => chipty::ModelId::Sprite,
	}
}

fn sprite_for_entity_args(edit: &chipcore::EditState, args: &chipty::EntityArgs) -> chipty::SpriteId {
	match args.kind {
		chipty::EntityKind::Player => sprite_for_player(args.face_dir, edit.get_terrain(args.pos)),
		chipty::EntityKind::PlayerNPC => sprite_for_playernpc(args.face_dir, edit.get_terrain(args.pos)),
		chipty::EntityKind::Chip => chipty::SpriteId::Chip,
		chipty::EntityKind::Socket => chipty::SpriteId::Socket,
		chipty::EntityKind::Block => chipty::SpriteId::DirtBlock,
		chipty::EntityKind::IceBlock => chipty::SpriteId::IceBlock,
		chipty::EntityKind::Flippers => chipty::SpriteId::Flippers,
		chipty::EntityKind::FireBoots => chipty::SpriteId::FireBoots,
		chipty::EntityKind::IceSkates => chipty::SpriteId::IceSkates,
		chipty::EntityKind::SuctionBoots => chipty::SpriteId::SuctionBoots,
		chipty::EntityKind::BlueKey => chipty::SpriteId::BlueKey,
		chipty::EntityKind::RedKey => chipty::SpriteId::RedKey,
		chipty::EntityKind::GreenKey => chipty::SpriteId::GreenKey,
		chipty::EntityKind::YellowKey => chipty::SpriteId::YellowKey,
		chipty::EntityKind::Thief => chipty::SpriteId::Thief,
		chipty::EntityKind::Bug => directional_sprite(args.face_dir, chipty::SpriteId::BugN, chipty::SpriteId::BugS, chipty::SpriteId::BugW, chipty::SpriteId::BugE),
		chipty::EntityKind::Tank => directional_sprite(args.face_dir, chipty::SpriteId::TankN, chipty::SpriteId::TankS, chipty::SpriteId::TankW, chipty::SpriteId::TankE),
		chipty::EntityKind::PinkBall => chipty::SpriteId::PinkBall,
		chipty::EntityKind::FireBall => chipty::SpriteId::FireballA,
		chipty::EntityKind::Glider => directional_sprite(args.face_dir, chipty::SpriteId::GliderN, chipty::SpriteId::GliderS, chipty::SpriteId::GliderW, chipty::SpriteId::GliderE),
		chipty::EntityKind::Walker => directional_sprite(args.face_dir, chipty::SpriteId::WalkerN, chipty::SpriteId::WalkerS, chipty::SpriteId::WalkerW, chipty::SpriteId::WalkerE),
		chipty::EntityKind::Teeth => directional_sprite(args.face_dir, chipty::SpriteId::TeethN, chipty::SpriteId::TeethS, chipty::SpriteId::TeethW, chipty::SpriteId::TeethE),
		chipty::EntityKind::Blob => chipty::SpriteId::Blob,
		chipty::EntityKind::Paramecium => directional_sprite(args.face_dir, chipty::SpriteId::ParameciumN, chipty::SpriteId::ParameciumS, chipty::SpriteId::ParameciumW, chipty::SpriteId::ParameciumE),
		chipty::EntityKind::Bomb => chipty::SpriteId::BombA,
	}
}

fn directional_sprite(face_dir: Option<chipty::Compass>, n: chipty::SpriteId, s: chipty::SpriteId, w: chipty::SpriteId, e: chipty::SpriteId) -> chipty::SpriteId {
	match face_dir {
		Some(chipty::Compass::Up) => n,
		Some(chipty::Compass::Down) => s,
		Some(chipty::Compass::Left) => w,
		Some(chipty::Compass::Right) => e,
		None => n,
	}
}

fn sprite_for_player(face_dir: Option<chipty::Compass>, terrain: chipty::Terrain) -> chipty::SpriteId {
	if matches!(terrain, chipty::Terrain::Water) {
		match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::PlayerSwimN,
			Some(chipty::Compass::Down) => chipty::SpriteId::PlayerSwimS,
			Some(chipty::Compass::Left) => chipty::SpriteId::PlayerSwimW,
			Some(chipty::Compass::Right) => chipty::SpriteId::PlayerSwimE,
			None => chipty::SpriteId::PlayerSwimIdle,
		}
	}
	else {
		match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::PlayerWalkN,
			Some(chipty::Compass::Down) => chipty::SpriteId::PlayerWalkS,
			Some(chipty::Compass::Left) => chipty::SpriteId::PlayerWalkW,
			Some(chipty::Compass::Right) => chipty::SpriteId::PlayerWalkE,
			None => chipty::SpriteId::PlayerWalkIdle,
		}
	}
}

fn sprite_for_playernpc(face_dir: Option<chipty::Compass>, terrain: chipty::Terrain) -> chipty::SpriteId {
	if matches!(terrain, chipty::Terrain::Water) {
		sprite_for_player(face_dir, terrain)
	}
	else if matches!(terrain, chipty::Terrain::Fire) {
		chipty::SpriteId::PlayerBurned
	}
	else {
		match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::PlayerWalkN,
			Some(chipty::Compass::Down) => chipty::SpriteId::PlayerWalkS,
			Some(chipty::Compass::Left) => chipty::SpriteId::PlayerWalkW,
			Some(chipty::Compass::Right) => chipty::SpriteId::PlayerWalkE,
			None => chipty::SpriteId::PlayerBurned,
		}
	}
}
