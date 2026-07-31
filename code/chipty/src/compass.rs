use super::*;

/// Movement and facing directions.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Compass {
	Up,
	Left,
	Down,
	Right,
}

impl Compass {
	#[inline]
	pub fn turn_left(self) -> Compass {
		match self {
			Compass::Up => Compass::Left,
			Compass::Left => Compass::Down,
			Compass::Down => Compass::Right,
			Compass::Right => Compass::Up,
		}
	}

	#[inline]
	pub fn turn_right(self) -> Compass {
		match self {
			Compass::Up => Compass::Right,
			Compass::Left => Compass::Up,
			Compass::Down => Compass::Left,
			Compass::Right => Compass::Down,
		}
	}

	#[inline]
	pub fn turn_around(self) -> Compass {
		match self {
			Compass::Up => Compass::Down,
			Compass::Left => Compass::Right,
			Compass::Down => Compass::Up,
			Compass::Right => Compass::Left,
		}
	}

	#[inline]
	pub fn to_vec(self) -> Vec2i {
		match self {
			Compass::Up => Vec2i(0, -1),
			Compass::Left => Vec2i(-1, 0),
			Compass::Down => Vec2i(0, 1),
			Compass::Right => Vec2i(1, 0),
		}
	}

	#[inline]
	pub fn from_vec(v: Vec2i) -> Option<Compass> {
		match (v.x.signum(), v.y.signum()) {
			(0, -1) => Some(Compass::Up),
			(-1, 0) => Some(Compass::Left),
			(0, 1) => Some(Compass::Down),
			(1, 0) => Some(Compass::Right),
			_ => None,
		}
	}
}

impl urandom::Distribution<Compass> for urandom::distr::StandardUniform {
	fn sample<R: urandom::Rng + ?Sized>(&self, rng: &mut urandom::Random<R>) -> Compass {
		match rng.random::<u32>() % 4 {
			0 => Compass::Up,
			1 => Compass::Left,
			2 => Compass::Down,
			_ => Compass::Right,
		}
	}
}
