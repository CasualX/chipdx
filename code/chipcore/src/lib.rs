/*!
Core chipgame gameplay code.
*/

use std::collections::HashMap;
use std::{cmp, fmt, mem, ops};
use cvmath::Vec2i;
use chipty::*;

mod cheats;
mod edit;
mod editstate;
mod entities;
mod entity;
mod entitymap;
mod event;
mod field;
mod gamestate;
mod hidden;
mod inbuf;
mod input;
mod movement;
mod playerstate;
mod quadtree;
mod random;
mod terrain;

pub use self::entities::*;
pub use self::entity::*;
pub use self::entitymap::*;
pub use self::event::*;
pub use self::editstate::*;
pub use self::field::*;
pub use self::gamestate::*;
use self::hidden::*;
pub use self::inbuf::*;
pub use self::input::*;
pub use self::movement::*;
pub use self::playerstate::*;
pub use self::quadtree::*;
pub use self::random::*;
use self::terrain::*;

/// Game ticks per second.
pub const FPS: i32 = 60;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TrapState {
	/// Trap is Closed and will trap any entity stepping on it.
	Active,
	/// Trap is Open and will not trap entities.
	Inactive,
}

#[derive(Default)]
pub struct ActionPhase {
	// Placeholder for future use
}

pub enum RngSeed {
	System,
	Manual(u64),
}

#[repr(transparent)]
pub struct FmtTime(pub i32);

impl FmtTime {
	#[inline]
	pub fn new(ticks: &i32) -> &FmtTime {
		unsafe { mem::transmute(ticks) }
	}
}

impl fmt::Display for FmtTime {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let &FmtTime(ticks) = self;

		let frames = ticks % FPS;
		let total_seconds = ticks / FPS;
		let seconds = total_seconds % 60;
		let minutes = total_seconds / 60;

		if minutes > 0 {
			write!(f, "{}:{:02}.{:02}", minutes, seconds, frames)
		}
		else {
			write!(f, "{}.{:02}", seconds, frames)
		}
	}
}
