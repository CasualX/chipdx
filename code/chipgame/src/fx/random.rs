use std::ops;

/// Shared random state for FxState.
#[derive(Clone)]
pub struct Random {
	rng: urandom::Random<urandom::rng::Xoshiro256Rng>,
}
impl ops::Deref for Random {
	type Target = urandom::Random<urandom::rng::Xoshiro256Rng>;
	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.rng
	}
}
impl ops::DerefMut for Random {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.rng
	}
}
impl Default for Random {
	#[inline]
	fn default() -> Self {
		Self { rng: urandom::rng::Xoshiro256Rng::new() }
	}
}
