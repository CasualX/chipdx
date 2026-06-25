use super::*;

#[derive(Clone)]
pub struct CameraShake {
	offset: Vec3f,
	time: f32,
	duration: f32,
	magnitude: f32,
}

impl Default for CameraShake {
	fn default() -> Self {
		Self {
			offset: Vec3f::ZERO,
			time: 0.0,
			duration: 0.0,
			magnitude: 0.0,
		}
	}
}

impl CameraShake {
	pub fn offset(&self) -> Vec3f {
		self.offset
	}

	pub fn add(&mut self, magnitude: f32, duration: f32) {
		if magnitude <= 0.0 || duration <= 0.0 {
			return;
		}
		self.magnitude = self.magnitude.max(magnitude);
		self.duration = self.duration.max(duration);
		self.time = self.duration;
	}

	pub fn update(&mut self, dt: f64, random: &mut Random) {
		if self.time <= 0.0 || self.duration <= 0.0 || self.magnitude <= 0.0 {
			self.clear();
			return;
		}

		self.time = f32::max(0.0, self.time - dt as f32);
		let envelope = (self.time / self.duration) as f32;
		let magnitude = self.magnitude * envelope * envelope;
		if magnitude <= 0.0 {
			self.clear();
			return;
		}

		self.offset = Vec3(random.range(-1.0..1.0), random.range(-1.0..1.0), 0.0) * magnitude;
		if self.time <= 0.0 {
			self.duration = 0.0;
			self.magnitude = 0.0;
		}
	}

	pub fn clear(&mut self) {
		self.offset = Vec3f::ZERO;
		self.time = 0.0;
		self.duration = 0.0;
		self.magnitude = 0.0;
	}

	pub fn attenuate(strength: f32, distance: f32, falloff: f32) -> f32 {
		if strength <= 0.0 {
			return 0.0;
		}
		if falloff <= 0.0 {
			return strength;
		}
		strength / (1.0 + distance.sqrt() * falloff)
	}
}
