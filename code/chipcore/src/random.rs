use super::*;

/// Source of game randomness.
#[derive(Clone)]
#[repr(transparent)]
pub struct Random {
	rand: urandom::Random<urandom::rng::Xoshiro256Rng>,
}

impl ops::Deref for Random {
	type Target = urandom::Random<urandom::rng::Xoshiro256Rng>;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.rand
	}
}

impl ops::DerefMut for Random {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.rand
	}
}

impl Default for Random {
	fn default() -> Self {
		Random {
			rand: urandom::rng::Xoshiro256Rng::new(),
		}
	}
}

impl Random {
	pub fn reseed(&mut self, seed: u64) {
		self.rand = urandom::rng::Xoshiro256Rng::from_seed_u64(seed);
	}
}
