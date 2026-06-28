use super::*;

pub use chipty::EntityKind;

/// Static entity data.
#[derive(Debug)]
pub struct EntityData {
	pub movement_phase: fn(&mut GameState, &mut MovementPhase, &mut Entity),
	pub action_phase: fn(&mut GameState, &mut ActionPhase, &mut Entity),
	pub terrain_phase: fn(&mut GameState, &mut TerrainPhase, &mut Entity),
	pub flags: SolidFlags,
	pub hidden: HiddenData,
}

/// Entity structure.
#[derive(Clone, Debug)]
pub struct Entity {
	pub data: &'static EntityData,
	pub handle: EntityHandle,
	pub kind: EntityKind,
	pub pos: Vec2i,
	pub base_spd: i32,
	pub face_dir: Option<Compass>,
	pub step_dir: Option<Compass>,
	pub step_spd: i32,
	pub step_time: i32,
	pub flags: u8,
}

impl Entity {
	#[inline]
	pub fn to_entity_args(&self) -> EntityArgs {
		EntityArgs {
			kind: self.kind,
			pos: self.pos,
			face_dir: self.face_dir,
		}
	}

	#[inline]
	pub fn is_trapped(&self) -> bool {
		self.flags & (EF_TRAPPED | EF_RELEASED) == EF_TRAPPED
	}

	/// Checks if the entity has moved in the current tick.
	#[inline]
	pub fn has_moved(&self, time: i32) -> bool {
		time == self.step_time && self.flags & EF_NEW_POS != 0
	}
}

/// Entity will be removed at the end of the current tick.
pub const EF_REMOVE: u8 = 1 << 0;
/// Entity is hidden under a block.
pub const EF_HIDDEN: u8 = 1 << 1;
/// Entity is trapped and cannot move.
pub const EF_TRAPPED: u8 = 1 << 2;
/// Entity has been released from a trap.
pub const EF_RELEASED: u8 = 1 << 3;
/// Entity has been forced to move.
pub const EF_MOMENTUM: u8 = 1 << 5;
/// Entity has a new position after try_move.
pub const EF_NEW_POS: u8 = 1 << 6;
/// Entity was moved by terrain.
pub const EF_TERRAIN_MOVE: u8 = 1 << 7;
