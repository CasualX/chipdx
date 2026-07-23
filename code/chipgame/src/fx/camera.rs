use super::*;

const TILE_SIZE: f32 = 32.0;
const CLASSIC_VISION_TILES: f32 = 4.5;
const WIDE_VISION_TILES: f32 = 6.5;
const NEAR: f32 = 10.0;
const FAR: f32 = 2000.0;

const CLASSIC_OFFSET: Vec3f = Vec3(0.0, 0.5 * TILE_SIZE, 150.0);
const WIDE_OFFSET: Vec3f = Vec3(0.0, 1.0 * TILE_SIZE, CLASSIC_OFFSET.z * WIDE_VISION_TILES / CLASSIC_VISION_TILES);
const EDITOR_OFFSET: Vec3f = Vec3f(0.0, 0.0 * TILE_SIZE, 400.0);
const SHAKE_DURATION: f32 = 0.35;

#[derive(Clone)]
pub struct PositionController {
	pub target: Vec2f,
}
impl PositionController {
	fn target(&self, _time: f64) -> Vec2f {
		self.target
	}
}

#[derive(Clone)]
pub struct FollowEntityController {
	pub master: chipcore::EntityHandle,
	pub move_src: Vec2<i32>,
	pub move_dest: Vec2<i32>,
	pub move_time: f64,
	pub move_spd: f32,
}
impl FollowEntityController {
	fn target(&self, time: f64) -> Vec2f {
		let t = if self.move_spd <= 0.0 { 1.0 } else { ((time - self.move_time) as f32).clamp(0.0, self.move_spd) / self.move_spd };
		let src = self.move_src.map(|c| c as f32 * TILE_SIZE + TILE_SIZE * 0.5);
		let dest = self.move_dest.map(|c| c as f32 * TILE_SIZE + TILE_SIZE * 0.5);
		let new_target = src.lerp(dest, t);
		new_target
	}
}

#[derive(Clone)]
pub enum Controller {
	FollowEntity(FollowEntityController),
	FreeRoam(PositionController),
}
impl Controller {
	pub fn follow_entity(&self) -> Option<&FollowEntityController> {
		match self {
			Controller::FollowEntity(follow_entity) => Some(follow_entity),
			_ => None,
		}
	}
	pub fn follow_entity_mut(&mut self) -> Option<&mut FollowEntityController> {
		match self {
			Controller::FollowEntity(follow_entity) => Some(follow_entity),
			_ => None,
		}
	}
	pub fn free_roam(&self) -> Option<&PositionController> {
		match self {
			Controller::FreeRoam(position) => Some(position),
			_ => None,
		}
	}
	pub fn free_roam_mut(&mut self) -> Option<&mut PositionController> {
		match self {
			Controller::FreeRoam(position) => Some(position),
			_ => None,
		}
	}
}
impl Controller {
	fn target(&self, time: f64) -> Vec2f {
		match self {
			Controller::FollowEntity(follow_entity) => follow_entity.target(time),
			Controller::FreeRoam(position) => position.target(time),
		}
	}
}

#[derive(Clone, Default)]
pub struct ProjectionMode {
	/// Whether we are in perspective mode or orthographic mode.
	///
	/// Determines whether blend is animated towards 1.0 or 0.0.
	pub perspective: bool,
	/// Blend between orthographic and perspective projection.
	pub blend: f32,
}
impl ProjectionMode {
	pub fn animate(&mut self) {
		if self.perspective {
			self.blend = f32::min(1.0, self.blend + 0.01);
		}
		else {
			self.blend = f32::max(0.0, self.blend - 0.01);
		}
	}
}

/// Camera state that can be saved and restored.
///
/// Used to transition between editor and play mode to preserve the camera position and zoom level.
#[derive(Copy, Clone, Debug)]
pub struct PlayCameraState {
	pub target: Vec2f,
	pub offset: Vec3f,
	pub blend: f32,
	pub vision_half_extent: f32,
}

#[derive(Clone)]
pub struct PlayCamera {
	/// Camera controller controls the camera target position.
	pub controller: Controller,
	/// Play field bounds.
	pub bounds: Bounds2f,

	/// When switching between controllers, add this offset to the target position and decay it to zero over time.
	transition_offset: Vec2f,
	/// Smooth camera look-at target in the ground plane.
	target: Vec2f,
	/// Camera offset from the target position.
	offset: Vec3f,
	/// Animate the camera offset towards this target.
	offset_target: Vec3f,
	zoom_mode: chipty::ZoomMode,
	vision_half_extent: f32,

	/// Blend between orthographic and perspective projection.
	blend: ProjectionMode,

	/// Camera shake.
	pub shake: CameraShake,
}

impl Default for PlayCamera {
	fn default() -> PlayCamera {
		PlayCamera {
			controller: Controller::FreeRoam(PositionController { target: Vec2f::ZERO }),
			bounds: Bounds2::ZERO,
			transition_offset: Vec2f::ZERO,
			target: Vec2f::ZERO,
			offset: WIDE_OFFSET,
			offset_target: WIDE_OFFSET,
			zoom_mode: chipty::ZoomMode::Wide,
			vision_half_extent: TILE_SIZE * WIDE_VISION_TILES,
			blend: ProjectionMode::default(),
			shake: CameraShake::default(),
		}
	}
}

impl PlayCamera {
	pub fn save_state(&self) -> PlayCameraState {
		PlayCameraState {
			target: self.target(),
			offset: self.offset,
			blend: self.blend.blend,
			vision_half_extent: self.vision_half_extent,
		}
	}

	pub fn load_state(&mut self, state: PlayCameraState, time: f64) {
		self.offset = state.offset;
		self.blend.blend = state.blend;
		self.vision_half_extent = state.vision_half_extent;
		self.transition_offset = state.target - self.raw_target(time);
		self.update_target(time);
	}

	pub fn setup(&self, screen_size: Vec2i) -> shade::d3::Camera {
		let shake_offset = self.shake.offset();
		let target = self.target().vec3(0.0) + shake_offset;
		let offset = self.get_offset();
		let position = target + offset;

		let focus_depth = offset.len();
		let aspect_ratio = screen_size.x as f32 / screen_size.y as f32;
		let fov_y = projection_fov_y(aspect_ratio, focus_depth);
		let corr = offset_correction(offset.y, offset.z, fov_y);
		let corr = Vec3(0.0, corr, 0.0);
		let position = position + corr;
		let target = target + corr;
		let view = Transform3f::look_at(position, target, -Vec3f::Y, Hand::LH);
		let blend = cvmath::scalar::smootherstep(0.0, 1.0, self.blend.blend);
		let projection = Mat4::blend_ortho_perspective(blend, focus_depth, fov_y, aspect_ratio, NEAR, FAR, (Hand::LH, Clip::NO));
		let view_proj = projection * view;
		let inv_view_proj = view_proj.inverse();
		shade::d3::Camera {
			viewport: Bounds2::vec(screen_size),
			aspect_ratio,
			position,
			view,
			near: NEAR,
			far: FAR,
			projection,
			view_proj,
			inv_view_proj,
			clip: Clip::NO,
		}
	}

	/// Keeps the logical camera source inside the level, inset by the active play vision range.
	fn clamp_target(&self, target: Vec2f) -> Vec2f {
		let margin = match self.zoom_mode {
			chipty::ZoomMode::Classic | chipty::ZoomMode::Wide => self.vision_half_extent,
			chipty::ZoomMode::Fit | chipty::ZoomMode::Editor => 0.0,
		};
		let inset = self.bounds.inset(margin);
		let center = self.bounds.center();
		target.max(inset.mins.min(center)).min(inset.maxs.max(center))
	}

	fn get_offset(&self) -> Vec3f {
		self.offset.set_y(self.offset.y * self.blend.blend)
	}

	/// Switch to a new controller and smoothly transition the camera.
	pub fn switch_controller(&mut self, controller: Controller, time: f64) {
		let old_target = self.target();
		self.controller = controller;
		self.transition_offset = old_target - self.raw_target(time);
		self.update_target(time);
	}

	/// Switch to a new controller and immediately jump the camera to the new target.
	pub fn set_controller(&mut self, controller: Controller, time: f64) {
		self.controller = controller;
		self.transition_offset = Vec2f::ZERO;
		self.update_target(time);
	}

	fn raw_target(&self, time: f64) -> Vec2f {
		// Override the controller target when in Fit mode
		if self.zoom_mode == chipty::ZoomMode::Fit {
			return self.bounds.center();
		}
		return self.controller.target(time);
	}

	fn update_target(&mut self, time: f64) {
		let target = self.raw_target(time) + self.transition_offset;
		self.target = self.clamp_target(target);
		// Keep free-roam input from accumulating invisibly past an edge
		if let Some(free_roam) = self.controller.free_roam_mut() {
			let correction = self.target - target;
			free_roam.target += correction;
		}
	}

	/// Returns the smooth camera look-at target position in the ground plane.
	pub fn target(&self) -> Vec2f {
		self.target
	}

	/// Switches to a free roam controller and pans the camera by the given delta.
	pub fn pan_free_roam(&mut self, delta: Vec2f, time: f64) {
		if self.zoom_mode == chipty::ZoomMode::Fit {
			return;
		}
		if let Some(free_roam) = self.controller.free_roam_mut() {
			free_roam.target += delta;
		}
		else {
			if self.zoom_mode == chipty::ZoomMode::Fit {
				return;
			}
			let target = self.target() + delta;
			let controller = PositionController { target };
			self.switch_controller(Controller::FreeRoam(controller), time);
		}
	}

	pub fn set_perspective(&mut self, perspective: bool) {
		self.blend.perspective = perspective;
	}

	pub fn set_zoom_mode(&mut self, zoom_mode: chipty::ZoomMode, animate: bool, time: f64) {
		let old_target = self.target();
		self.zoom_mode = zoom_mode;
		self.offset_target = match zoom_mode {
			chipty::ZoomMode::Wide => WIDE_OFFSET,
			chipty::ZoomMode::Classic => CLASSIC_OFFSET,
			chipty::ZoomMode::Fit => WIDE_OFFSET,
			chipty::ZoomMode::Editor => EDITOR_OFFSET,
		};
		if !animate {
			self.offset = self.offset_target;
			self.vision_half_extent = match self.zoom_mode {
				chipty::ZoomMode::Classic | chipty::ZoomMode::Wide => zoom_vision_half_extent(self.offset.z),
				chipty::ZoomMode::Fit | chipty::ZoomMode::Editor => level_vision_half_extent(&self.bounds),
			};
		}

		let new_target = self.raw_target(time);
		self.transition_offset = if animate { old_target - new_target } else { Vec2f::ZERO };
		self.update_target(time);
	}

	/// Manually zooms the camera.
	pub fn zoom_by(&mut self, delta: f32, min: f32, max: f32) {
		self.offset_target.z = (self.offset_target.z + delta).clamp(min, max);
		self.offset.z = self.offset_target.z;
	}

	pub fn vision_clip(&self) -> Option<(Vec2f, f32)> {
		Some((self.target(), self.vision_half_extent))
	}

	pub fn add_shake_at(&mut self, pos: Vec3f, strength: f32, falloff: f32) {
		let distance = self.target().vec3(0.0).distance(pos);
		let magnitude = CameraShake::attenuate(strength, distance, falloff);
		self.shake.add(magnitude, SHAKE_DURATION);
	}

	// Update blend over time
	pub fn animate_blend(&mut self) {
		self.blend.animate();
	}

	fn fit_offset_target(&self, aspect_ratio: f32) -> Vec3f {
		let fov_y = projection_fov_y(aspect_ratio, WIDE_OFFSET.z);
		let padding = TILE_SIZE;
		let half_h = (self.bounds.height() * 0.5 + padding).max(1.0);
		let half_w = (self.bounds.width() * 0.5 + padding).max(1.0);
		let tan_half_fov_y = (fov_y * 0.5).tan();
		let tan_half_fov_x = tan_half_fov_y * aspect_ratio;
		let focus_depth = (half_h / tan_half_fov_y).max(half_w / tan_half_fov_x).max(NEAR * 2.0);
		let y = WIDE_OFFSET.y * self.blend.blend;
		let z = (focus_depth * focus_depth - y * y).max(1.0).sqrt();
		Vec3(0.0, y, z)
	}

	pub fn animate_position(&mut self, time: f64, dt: f64, screen_size: Vec2i) {
		if self.zoom_mode == chipty::ZoomMode::Fit {
			let aspect_ratio = screen_size.x as f32 / screen_size.y as f32;
			self.offset_target = self.fit_offset_target(aspect_ratio);
		}

		self.offset = self.offset.exp_decay(self.offset_target, 10.0, dt as f32);
		let vision_target = match self.zoom_mode {
			chipty::ZoomMode::Classic | chipty::ZoomMode::Wide => zoom_vision_half_extent(self.offset.z),
			chipty::ZoomMode::Fit | chipty::ZoomMode::Editor => level_vision_half_extent(&self.bounds),
		};
		self.vision_half_extent = Vec2(self.vision_half_extent, 0.0).exp_decay(Vec2(vision_target, 0.0), 10.0, dt as f32).x;
		self.transition_offset = self.transition_offset.exp_decay(Vec2f::ZERO, 10.0, dt as f32);
		if self.transition_offset.len() < 0.5 {
			self.transition_offset = Vec2f::ZERO;
		}
		self.update_target(time);
	}
}

fn level_vision_half_extent(bounds: &Bounds2f) -> f32 {
	let width = bounds.width();
	let height = bounds.height();
	(width * width + height * height).sqrt() * 0.5 + TILE_SIZE * 2.0
}

fn zoom_vision_half_extent(offset_z: f32) -> f32 {
	fn zoom_lerp_t(offset_z: f32) -> f32 {
		((offset_z - CLASSIC_OFFSET.z) / (WIDE_OFFSET.z - CLASSIC_OFFSET.z)).clamp(0.0, 1.0)
	}
	let t = zoom_lerp_t(offset_z);
	let classic = TILE_SIZE * CLASSIC_VISION_TILES;
	let wide = TILE_SIZE * WIDE_VISION_TILES;
	cvmath::lerp(classic, wide, t)
}

fn projection_fov_y(aspect_ratio: f32, _focus_depth: f32) -> Anglef {
	let aspect_ratio = aspect_ratio.max(0.001);
	let ref_half_short_side = TILE_SIZE * CLASSIC_VISION_TILES;
	let ref_half_fov_tan = ref_half_short_side / CLASSIC_OFFSET.z;
	let portrait_half_fov_tan = ref_half_fov_tan / aspect_ratio;
	let half_fov_tan = ref_half_fov_tan.max(portrait_half_fov_tan);
	Angle::atan(half_fov_tan) * 2.0
}

// When looking at the scene from an angle more space is visible above than below the target.
// This function computes an offset to apply to the camera and target position to keep the space above and below the target more balanced.
fn offset_correction(dx: f32, dy: f32, fov_y: Anglef) -> f32 {
	// Note: atan2 is defined as atan2(opposite, adjacent) which is why the arguments are swapped.
	let angle = Anglef::atan2(dx, dy);
	let angle_top = angle + fov_y * 0.5;
	let angle_bot = angle - fov_y * 0.5;

	let hit_top = dy * angle_top.tan();
	let hit_bot = dy * angle_bot.tan();

	((hit_top + hit_bot) * 0.5) - dx
}

#[test]
fn test_offset_correction() {
	let dy = 200.0;
	let fov_y = Angle::deg(90.0);

	let corr1 = offset_correction(-32.0, dy, fov_y);
	assert_eq!(corr1.round(), -34.0);
	let corr2 = offset_correction(0.0, dy, fov_y);
	assert_eq!(corr2, 0.0);
}

#[test]
fn test_projection_fov_y_anchors_short_side_to_four_and_a_half_tiles() {
	let focus_depth = CLASSIC_OFFSET.z;
	let landscape = projection_fov_y(16.0 / 9.0, CLASSIC_OFFSET.z);
	let landscape_half_height = (landscape * 0.5).tan() * focus_depth;
	assert!((landscape_half_height - TILE_SIZE * CLASSIC_VISION_TILES).abs() < 0.001);

	let portrait = projection_fov_y(9.0 / 16.0, CLASSIC_OFFSET.z);
	let portrait_half_height = (portrait * 0.5).tan() * focus_depth;
	let portrait_half_width = portrait_half_height * (9.0 / 16.0);
	assert!((portrait_half_width - TILE_SIZE * CLASSIC_VISION_TILES).abs() < 0.001);
}

#[test]
fn test_wide_offset_matches_wide_vision_tiles() {
	let half_fov_tan = (projection_fov_y(16.0 / 9.0, CLASSIC_OFFSET.z) * 0.5).tan();
	let wide_half_height = half_fov_tan * WIDE_OFFSET.z;
	assert!((wide_half_height - TILE_SIZE * WIDE_VISION_TILES).abs() < 0.001);
}
