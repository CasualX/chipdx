use super::*;

const SCOUT_INPUT_HOLD: f64 = 1.5 / chipcore::FPS as f64;

#[derive(Clone)]
pub struct BearTrapState {
	pub state: chipcore::TrapState,
	pub handle: render::ObjectHandle,
}

#[derive(Clone, Default)]
pub struct FxState {
	pub game: chipcore::GameState,
	pub camera: PlayCamera,
	pub time: f64,
	pub dt: f64,
	pub random: Random,
	pub render: render::RenderState,

	pub game_objects: HashMap<chipcore::EntityHandle, render::ObjectHandle>,
	pub hidden_objects: HashMap<chipcore::EntityHandle, render::ObjectHandle>,
	pub fire_sprites: HashMap<Vec2i, render::ObjectHandle>,
	pub hidden_fire: HashMap<Vec2i, render::ObjectHandle>,
	pub toggle_walls: HashMap<Vec2i, render::ObjectHandle>,
	pub mirage_walls: HashMap<Vec2i, render::ObjectHandle>,
	pub fake_blue_walls: HashMap<Vec2i, render::ObjectHandle>,
	pub water_hazards: HashMap<Vec2i, render::ObjectHandle>,
	pub traps: HashMap<Vec2i, BearTrapState>,

	pub level_number: i32,
	pub next_level_load: f64,
	pub game_start_time: f64,
	pub game_realtime: f32,
	pub game_over: Option<chipcore::GameOverReason>,
	pub pause_pressed: bool,
	pub hud_enabled: bool,
	pub is_preview: bool,
	pub step_mode: bool,
	pub assist_dist: i32,
	pub darken: bool,
	pub darken_time: f64,
	pub events: Vec<FxEvent>,
	pub replay_inputs: Option<Vec<u8>>,
	pub warps_set: i32,
	pub warps_used: i32,
	pub unpauses: i32,
	pub scout_active: bool,
	pub scout_dir_until: [f64; 4],
	pub scout_speed: f32,
}

impl cvar::IVisit for FxState {
	fn visit(&mut self, f: &mut dyn FnMut(&mut dyn cvar::INode)) {
		self.game.visit(f);
	}
}

impl FxState {
	/// Creates a new FxState initialized for the given level.
	pub fn new(level_number: i32, level_dto: &chipty::LevelDto, rng_seed: chipcore::RngSeed, tiles: &'static [render::TileGfx]) -> Box<FxState> {
		let mut fx = Box::new(FxState::default());
		fx.level_number = level_number;
		fx.render.tiles = tiles;
		fx.hud_enabled = true;
		fx.darken = true;
		fx.darken_time = -1.0;
		fx.assist_dist = 2;

		// Parse the level data into the game state
		fx.game.parse(level_dto, rng_seed);
		fx.game.time_state = chipcore::TimeState::Waiting;
		let controller = FollowEntityController {
			master: fx.game.ps.master,
			move_src: Vec2::ZERO,
			move_dest: Vec2::ZERO,
			move_time: 0.0,
			move_spd: 1.0,
		};
		fx.camera.set_controller(Controller::FollowEntity(controller), 0.0);
		fx.camera.bounds.mins = Vec2::ZERO;
		fx.camera.bounds.maxs = Vec2(fx.game.field.width as f32 * 32.0, fx.game.field.height as f32 * 32.0);

		// Initialize the render field based on the game state
		fx.render.field.width = fx.game.field.width;
		fx.render.field.height = fx.game.field.height;
		fx.render.field.terrain.extend_from_slice(&fx.game.field.terrain);
		for y in 0..fx.game.field.height {
			for x in 0..fx.game.field.width {
				let pos = Vec2(x, y);
				let index = (y * fx.game.field.width + x) as usize;
				let terrain = fx.game.field.terrain[index];
				match terrain {
					chipty::Terrain::Fire => handlers::create_fire(&mut fx, pos),
					chipty::Terrain::ToggleFloor => handlers::create_toggle_wall(&mut fx, pos, false),
					chipty::Terrain::ToggleWall => handlers::create_toggle_wall(&mut fx, pos, true),
					chipty::Terrain::HiddenWall => handlers::create_wall_mirage(&mut fx, pos),
					chipty::Terrain::InvisibleWall => handlers::create_wall_mirage(&mut fx, pos),
					chipty::Terrain::WaterHazard => handlers::create_water_hazard(&mut fx, pos),
					chipty::Terrain::FakeBlueWall => handlers::create_fake_blue_wall(&mut fx, pos),
					chipty::Terrain::BearTrap => handlers::create_bear_trap(&mut fx, pos),
					_ => {}
				}
			}
		}

		// Sync initial game state
		fx.sync();
		return fx;
	}

	pub fn scout(&mut self) {
		self.game.time_state = chipcore::TimeState::Paused;
		self.events.push(FxEvent::ScoutMode);
		self.hud_enabled = true;
		self.scout_init(true);
	}
	pub fn pause(&mut self) {
		self.game.time_state = chipcore::TimeState::Paused;
		self.events.push(FxEvent::PauseGame);
		self.hud_enabled = false;
		self.scout_init(false);
	}
	pub fn resume(&mut self) {
		if matches!(self.game.time_state, chipcore::TimeState::Paused) {
			self.game.time_state = chipcore::TimeState::Running;
			self.events.push(FxEvent::ResumePlay);
			self.hud_enabled = true;
			self.unpauses += 1;
			self.scout_init(false);

			// Center camera on player again
			let master = self.game.ps.master;
			if let Some(entity) = self.game.ents.get(master) {
				let controller = FollowEntityController {
					master,
					move_src: entity.pos,
					move_dest: entity.pos,
					move_time: self.time,
					move_spd: 8.0 / chipcore::FPS as f32,
				};
				self.camera.switch_controller(Controller::FollowEntity(controller), self.time);
			}
		}
	}
	pub fn think(&mut self, input: &chipcore::Input, menu_active: bool) {
		if menu_active {
			self.scout_handle_input(input);
			return;
		}

		// Handle pause/scout input
		if !self.game.is_game_over() {
			if input.start && !self.pause_pressed {
				self.pause();
			}
			else if input.select && !self.pause_pressed {
				self.scout();
			}
			self.pause_pressed = input.start || input.select;
		}

		// Handle replay inputs
		let player_input = *input;
		if let Some(_) = &mut self.replay_inputs {
			if player_input.has_directional_input() {
				self.replay_inputs = None;
			}
		}
		let replay_input = self.replay_inputs.as_ref().and_then(|inputs| {
			inputs.get(self.game.time as usize).cloned().map(chipcore::Input::decode)
		});
		let input = replay_input.unwrap_or(player_input);

		// Handle step mode
		let mut run_tick = true;
		if self.step_mode {
			run_tick = self.game.should_tick_step_mode(&input);
			// Reset input buffering to avoid stale inputs
			// FIXME: This causes desync when replaying inputs
			// if !run_tick {
			// 	self.game.input_reset();
			// }
		}

		if run_tick {
			self.game.tick(&input);
			self.sync();
		}

		self.update_camera_master();
		self.update_timer();
		self.update_traps();

		let fade_invisible = FadeValues { near_alpha: 0.75, far_alpha: 0.0 };
		let fade_visible = FadeValues { near_alpha: 0.75, far_alpha: 1.0 };

		// Update objects based on player proximity
		fade_assist_dist(&self.mirage_walls, &mut self.render.objects, &fade_invisible, |&pos|
			chipcore::ps_nearest_ent(&self.game, pos).map(|player| player.pos.distance_hat(pos) <= self.assist_dist));
		fade_assist_dist(&self.hidden_fire, &mut self.render.objects, &fade_invisible, |&pos|
			chipcore::ps_nearest_ent(&self.game, pos).map(|player| player.pos.distance_hat(pos) <= self.assist_dist));
		fade_assist_dist(&self.fake_blue_walls, &mut self.render.objects, &fade_visible, |&pos|
			chipcore::ps_nearest_ent(&self.game, pos).map(|player| player.pos.distance_hat(pos) <= self.assist_dist));
		fade_assist_dist(&self.hidden_objects, &mut self.render.objects, &fade_invisible, |&ehandle| {
			let ent = self.game.ents.get(ehandle)?;
			// Hidden objects are kind of scuffed... Some special rules:
			// If the entity is hidden inside a wall, make it always visible
			let terrain = self.game.field.get_terrain(ent.pos);
			let is_wall = matches!(terrain,
				chipty::Terrain::Wall | chipty::Terrain::HiddenWall | chipty::Terrain::InvisibleWall |
				chipty::Terrain::RealBlueWall |// chipty::Terrain::FakeBlueWall |
				chipty::Terrain::BlueLock | chipty::Terrain::RedLock | chipty::Terrain::GreenLock | chipty::Terrain::YellowLock |
				chipty::Terrain::ToggleWall | chipty::Terrain::ToggleFloor |
				chipty::Terrain::CloneBlockN | chipty::Terrain::CloneBlockW | chipty::Terrain::CloneBlockS | chipty::Terrain::CloneBlockE |
				chipty::Terrain::CloneMachine);
			if is_wall {
				return Some(true);
			}
			let player = chipcore::ps_nearest_ent(&self.game, ent.pos)?;
			let dist = player.pos.distance_hat(ent.pos);
			Some(dist <= self.assist_dist)
		});
	}
	/// Process game events and update FX state accordingly.
	pub fn sync(&mut self) {
		for ev in &self.game.events.take() {
			eprintln!("GameEvent: {:?}", ev);
			match ev {
				&chipcore::GameEvent::EntityCreated { entity, kind } => handlers::entity_created(self, entity, kind),
				&chipcore::GameEvent::EntityRemoved { entity, kind } => handlers::entity_removed(self, entity, kind),
				&chipcore::GameEvent::EntityStep { entity } => handlers::entity_step(self, entity),
				&chipcore::GameEvent::EntityTurn { entity } => handlers::entity_face_dir(self, entity),
				&chipcore::GameEvent::EntityHidden { entity, hidden } => handlers::entity_hidden(self, entity, hidden),
				&chipcore::GameEvent::EntityTeleport { entity } => handlers::entity_teleport(self, entity),
				&chipcore::GameEvent::PlayerGameOver { entity, reason } => handlers::player_game_over(self, entity, reason),
				&chipcore::GameEvent::PlayerActivity { entity } => handlers::player_activity(self, entity),
				&chipcore::GameEvent::PlayerPush { entity } => handlers::player_push(self, entity),
				&chipcore::GameEvent::PlayerBump { entity } => handlers::player_push(self, entity),
				&chipcore::GameEvent::ItemsThief { player: _, boots } => handlers::items_thief(self, boots),
				&chipcore::GameEvent::LockOpened { pos, key } => handlers::lock_opened(self, pos, key),
				&chipcore::GameEvent::FireHidden { pos, hidden } => handlers::fire_hidden(self, pos, hidden),
				&chipcore::GameEvent::TerrainUpdated { pos, old, new } => handlers::terrain_updated(self, pos, old, new),
				&chipcore::GameEvent::GameOver { reason } => handlers::game_over(self, reason),
				&chipcore::GameEvent::SoundFx { sound } => self.events.push(FxEvent::PlaySound(sound)),
				&chipcore::GameEvent::BombExplode { pos } => handlers::bomb_explode(self, pos),
				&chipcore::GameEvent::WaterSplash { pos } => handlers::effect(self, pos, render::EffectType::Splash),
				_ => {}
			}
		}
	}
	pub fn draw(&mut self, g: &mut shade::Graphics, resx: &Resources, time: f64) {
		let dt = time - self.time;
		self.time = time;
		self.dt = dt;
		let ctx = render::UpdateCtx { time: self.time, dt: self.dt };

		if self.game.time != 0 {
			self.camera.animate_blend();
		}
		self.scout_camera(ctx.dt);
		self.camera.animate_position(self.time, ctx.dt, resx.viewport.size());
		self.camera.shake.update(ctx.dt, &mut self.random);
		self.render.update(&ctx);

		self.draw_shadow_map(g, resx);
		let camera = self.camera.setup(resx.viewport.size());
		let vision_clip = self.camera.vision_clip();
		self.render.draw(g, resx, &camera, time, vision_clip);

		if self.hud_enabled {
			g.begin(&shade::BeginArgs::Immediate {
				viewport: resx.viewport,
				color: &[resx.backcolor()],
				levels: None,
				depth: Some(resx.backdepth()),
			});
			self.render_ui(g, resx, time);
			g.end();
		}
	}

	fn update_camera_master(&mut self) {
		if self.game.field.camera_triggers.is_empty() {
			return;
		}

		let Some(player) = self.game.ents.get(self.game.ps.master) else {
			return;
		};

		let ehandle = self.game.field.camera_triggers.iter().find_map(|camera_trigger| {
			let ehandle = chipcore::EntityHandle::from(camera_trigger.entity_index);
			let Some(entity) = self.game.ents.get(ehandle) else {
				return None;
			};
			if camera_trigger.player_pos != player.pos {
				return None;
			}
			if camera_trigger.entity_kind != entity.kind {
				return None;
			}
			Some(ehandle)
		}).unwrap_or(self.game.ps.master);

		let camera_master = self.camera.controller.follow_entity().map(|follow| follow.master);
		if camera_master != Some(ehandle) {
			if let Some(entity) = self.game.ents.get(ehandle) {
				self.camera.switch_controller(Controller::FollowEntity(FollowEntityController {
					master: ehandle,
					move_src: entity.pos,
					move_dest: entity.pos,
					move_time: self.time,
					move_spd: 8.0 / chipcore::FPS as f32,
				}), self.time);
			}
		}
	}

	fn update_timer(&mut self) {
		if self.game_start_time == 0.0 && self.game.time > 0 {
			self.game_start_time = self.time;
		}

		if self.next_level_load != 0.0 && self.time > self.next_level_load {
			self.next_level_load = 0.0;
			let event = if matches!(self.game_over, Some(chipcore::GameOverReason::LevelComplete)) {
				FxEvent::LevelComplete
			}
			else {
				FxEvent::GameOver
			};
			self.events.push(event);
		}
	}

	fn update_traps(&mut self) {
		for (&dest, trap) in &mut self.traps {
			let current_state = self.game.get_trap_state(dest);
			if trap.state == current_state {
				continue;
			}
			trap.state = current_state;

			// Animate the BearTrap sprite
			if let Some(obj) = self.render.objects.get_mut(trap.handle) {
				let sprite = match trap.state {
					chipcore::TrapState::Active => chipty::SpriteId::BearTrapActiveA,
					chipcore::TrapState::Inactive => chipty::SpriteId::BearTrapInactiveA,
				};
				obj.data.sprite = sprite;
				obj.anim.anims.push(render::AnimState::AnimSeq(render::SpriteAnimSeq {
					start_time: self.time,
					frame_count: 5,
					frame_rate: 40.0,
				}));
			}
		}
	}

	fn scout_handle_input(&mut self, input: &chipcore::Input) {
		self.scout_speed = if input.a { 5.0 } else { 2.0 };

		let scout_until = self.time + SCOUT_INPUT_HOLD;
		if input.up {
			self.scout_dir_until[chipty::Compass::Up as usize] = scout_until;
		}
		if input.right {
			self.scout_dir_until[chipty::Compass::Right as usize] = scout_until;
		}
		if input.down {
			self.scout_dir_until[chipty::Compass::Down as usize] = scout_until;
		}
		if input.left {
			self.scout_dir_until[chipty::Compass::Left as usize] = scout_until;
		}
	}
	fn scout_camera(&mut self, dt: f64) {
		if !self.scout_active || dt <= 0.0 {
			return;
		}
		let now = self.time;
		let north = if now <= self.scout_dir_until[chipty::Compass::Up as usize] { 1 } else { 0 };
		let south = if now <= self.scout_dir_until[chipty::Compass::Down as usize] { 1 } else { 0 };
		let east = if now <= self.scout_dir_until[chipty::Compass::Right as usize] { 1 } else { 0 };
		let west = if now <= self.scout_dir_until[chipty::Compass::Left as usize] { 1 } else { 0 };
		let dir = Vec2(east - west, south - north);
		if dir == Vec2::ZERO {
			return;
		}
		let speed = self.scout_speed;
		if speed <= 0.0 {
			return;
		}
		let pixels_per_second = speed * chipcore::FPS as f32;
		let delta = dir.cast::<f32>() * (pixels_per_second * dt as f32);
		self.camera.pan_free_roam(delta, self.time);
	}
	fn scout_init(&mut self, scout_active: bool) {
		self.scout_active = scout_active;
		self.scout_dir_until = [0.0; 4];
		self.scout_speed = 0.0;
	}

	fn draw_shadow_map(&mut self, g: &mut shade::Graphics, resx: &Resources) {
		const SIZE: i32 = 2048;
		let info = shade::Texture2DInfo {
			format: shade::TextureFormat::Depth24,
			width: SIZE,
			height: SIZE,
			props: shade::TextureProps {
				mip_levels: 1,
				usage: shade::TextureUsage!(DEPTH_STENCIL_TARGET | SAMPLED),
				filter_min: shade::TextureFilter::Nearest,
				filter_mag: shade::TextureFilter::Nearest,
				wrap_u: shade::TextureWrap::Edge,
				wrap_v: shade::TextureWrap::Edge,
				border_color: [1.0, 1.0, 1.0, 1.0],
				..Default::default()
			},
		};
		g.texture2d_ensure(&mut self.render.shadow_map, &info);

		let viewport = cvmath::Bounds2!(0, 0, SIZE, SIZE);

		let light_target = self.camera.target().vec3(0.0);
		let light_pos = light_target + Vec3f::new(-300.0, 300.0, 300.0);
		let view = cvmath::Transform3f::look_at(light_pos, light_target, -Vec3f::Y, Hand::LH);
		let near = 10.0;
		let far = 1000.0;
		let projection = cvmath::Mat4f::ortho(-500.0, 500.0, -500.0, 500.0, near, far, (Hand::LH, Clip::NO));
		let view_proj = projection * view;
		self.render.light_matrix = view_proj;
		let inv_view_proj = view_proj.inverse();
		let camera = shade::d3::Camera {
			viewport,
			aspect_ratio: 1.0,
			position: light_pos,
			view,
			near,
			far,
			projection,
			view_proj,
			inv_view_proj,
			clip: Clip::NO,
		};

		g.begin(&shade::BeginArgs::Immediate {
			color: &[],
			levels: None,
			depth: Some(self.render.shadow_map()),
			viewport,
		});
		g.clear(&shade::ClearArgs {
			depth: Some(1.0),
			..Default::default()
		});
		let vision_clip = self.camera.vision_clip();
		self.render.draw_field(g, resx, &camera, self.time, true, vision_clip);
		g.end();
	}
}

struct FadeValues {
	near_alpha: f32,
	far_alpha: f32,
}

fn fade_assist_dist<T, F: Fn(&T) -> Option<bool>>(
	map: &HashMap<T, render::ObjectHandle>,
	objects: &mut render::ObjectMap,
	values: &FadeValues,
	f: F,
) {
	for (key, &obj_handle) in map {
		let Some(obj) = objects.get_mut(obj_handle) else { continue };
		let Some(nearby) = f(key) else { continue };
		let target_alpha = if nearby { values.near_alpha } else { values.far_alpha };
		fade_to(obj, target_alpha, 4.0);
	}
}

fn fade_to(obj: &mut render::Object, target_alpha: f32, fade_spd: f32) {
	for anim in &mut obj.anim.anims {
		if let render::AnimState::FadeTo(fade) = anim {
			fade.target_alpha = target_alpha;
			fade.fade_spd = fade_spd;
			return;
		}
	}
	obj.anim.anims.push(render::AnimState::FadeTo(render::FadeTo { target_alpha, fade_spd }));
}

pub fn draw_entity_order(fx: &FxState, g: &mut shade::Graphics, resx: &Resources, camera: &shade::d3::Camera) {
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
	for (&ehandle, &obj_handle) in &fx.game_objects {
		let Some(index) = ehandle.index() else { continue };
		let Some(obj) = fx.render.objects.get(obj_handle) else { continue };
		let Some(pos) = camera.world_to_viewport(obj.data.pos + Vec3(16.0, 16.0, 0.0)) else { continue };
		tbuf.text_box(&resx.font, &scribe, &cvmath::Bounds2::point(pos, Vec2::ZERO), shade::d2::TextAlign::MiddleCenter, &format!("{}", index));
	}

	tbuf.draw(g);
}
