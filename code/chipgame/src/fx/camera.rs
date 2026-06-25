use super::*;

const TILE_SIZE: f32 = 32.0;
const CLASSIC_VISION_TILES: f32 = 4.5;
const WIDE_VISION_TILES: f32 = 6.5;
const NEAR: f32 = 10.0;
const FAR: f32 = 2000.0;

const CLASSIC_OFFSET_Z: f32 = 150.0;
const WIDE_OFFSET_Z: f32 = CLASSIC_OFFSET_Z * WIDE_VISION_TILES / CLASSIC_VISION_TILES;

const WIDE_OFFSET: Vec3f = Vec3(0.0, 1.0 * 32.0, WIDE_OFFSET_Z);
const CLASSIC_OFFSET: Vec3f = Vec3(0.0, 0.5 * 32.0, CLASSIC_OFFSET_Z);
const EDITOR_OFFSET: Vec3f = Vec3f(0.0, 0.0 * 32.0, 400.0);
const SHAKE_DURATION: f32 = 0.35;

#[derive(Clone)]
pub struct PlayCamera {
	// Camera offset from the object
	offset: Vec3f,
	/// Animate the camera offset towards this target
	offset_target: Vec3f,

	// Camera look at target
	pub target: Vec3f,

	// Camera position
	// Slowly tracks target + offset
	pub position: Vec3f,

	// Blend between orthographic and perspective projection
	blend: f32,
	perspective: bool,
	zoom_mode: chipty::ZoomMode,

	pub master: chipcore::EntityHandle,
	pub move_src: Vec2<i32>,
	pub move_dest: Vec2<i32>,
	pub move_time: f64,
	pub move_spd: f32,
	pub move_teleport: bool,

	pub shake: CameraShake,
}

impl Default for PlayCamera {
	fn default() -> PlayCamera {
		PlayCamera {
			offset: WIDE_OFFSET,
			offset_target: WIDE_OFFSET,
			target: Vec3f::ZERO,
			position: Vec3f::ONE,
			blend: 0.0,
			perspective: false,
			zoom_mode: chipty::ZoomMode::Wide,
			master: chipcore::EntityHandle::INVALID,
			move_src: Vec2::ZERO,
			move_dest: Vec2::ZERO,
			move_time: 0.0,
			move_spd: 1.0,
			move_teleport: true,
			shake: CameraShake::default(),
		}
	}
}

impl PlayCamera {
	pub fn setup(&self, screen_size: Vec2i) -> shade::d3::Camera {
		let aspect_ratio = screen_size.x as f32 / screen_size.y as f32;
		let shake_offset = self.shake.offset();
		let target = self.target + shake_offset;
		let pos = target + self.get_offset();
		let focus_depth = pos.distance(target);
		let fov_y = projection_fov_y(aspect_ratio, focus_depth);
		let corr = offset_correction(pos.y - target.y, pos.z, fov_y);
		let corr = Vec3(0.0, corr, 0.0);
		let position = self.position + shake_offset + corr;
		let target = target + corr;
		let view = Transform3f::look_at(position, target, -Vec3f::Y, Hand::LH);
		let projection = Mat4::blend_ortho_perspective(self.blend, focus_depth, fov_y, aspect_ratio, NEAR, FAR, (Hand::LH, Clip::NO));
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

	fn get_offset(&self) -> Vec3f {
		self.offset.set_y(self.offset.y * self.blend)
	}

	pub fn set_perspective(&mut self, perspective: bool) {
		self.perspective = perspective;
	}

	pub fn set_zoom_mode(&mut self, zoom_mode: chipty::ZoomMode, animate: bool) {
		self.zoom_mode = zoom_mode;
		self.offset_target = match zoom_mode {
			chipty::ZoomMode::Wide => WIDE_OFFSET,
			chipty::ZoomMode::Classic => CLASSIC_OFFSET,
			chipty::ZoomMode::Editor => EDITOR_OFFSET,
		};
		if !animate {
			self.offset = self.offset_target;
		}
	}

	pub fn zoom_by(&mut self, delta: f32, min: f32, max: f32) {
		self.offset_target.z = (self.offset_target.z + delta).clamp(min, max);
		self.offset.z = self.offset_target.z;
	}

	pub fn set_target(&mut self, pos: Vec3f) {
		self.target = pos;
	}

	pub fn vision_clip(&self) -> Option<(Vec2f, f32)> {
		let half_extent = match self.zoom_mode {
			chipty::ZoomMode::Classic | chipty::ZoomMode::Wide => {
				fn zoom_lerp_t(offset_z: f32) -> f32 {
					((offset_z - CLASSIC_OFFSET_Z) / (WIDE_OFFSET_Z - CLASSIC_OFFSET_Z)).clamp(0.0, 1.0)
				}
				let t = zoom_lerp_t(self.get_offset().z);
				let classic = TILE_SIZE * CLASSIC_VISION_TILES;
				let wide = TILE_SIZE * WIDE_VISION_TILES;
				cvmath::lerp(classic, wide, t)
			}
			chipty::ZoomMode::Editor => return None,
		};
		Some((self.target.xy(), half_extent))
	}

	pub fn add_shake_at(&mut self, pos: Vec3f, strength: f32, falloff: f32) {
		let distance = self.target.distance(pos);
		let magnitude = CameraShake::attenuate(strength, distance, falloff);
		self.shake.add(magnitude, SHAKE_DURATION);
	}

	// Update blend over time
	pub fn animate_blend(&mut self) {
		if self.perspective {
			self.blend = f32::min(1.0, self.blend + 0.01);
		}
		else {
			self.blend = f32::max(0.0, self.blend - 0.01);
		}
	}

	pub fn animate_position(&mut self, dt: f64) {
		self.position = self.target + self.get_offset();
		self.offset = self.offset.exp_decay(self.offset_target, 10.0, dt as f32);
	}

	pub fn animate_move(&mut self, time: f64) {
		let t = f32::min(1.0, (time - self.move_time) as f32 / self.move_spd);
		let src = self.move_src.map(|c| c as f32 * 32.0 + 16.0).vec3(0.0);
		let dest = self.move_dest.map(|c| c as f32 * 32.0 + 16.0).vec3(0.0);
		let new_target = src.lerp(dest, t);
		self.target = new_target;
		if self.move_teleport {
			self.position = new_target + self.get_offset();
			self.move_teleport = false;
		}
	}
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
