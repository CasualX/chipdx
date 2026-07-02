use super::*;

pub const FIELD_MIN_WIDTH: i32 = 3;
pub const FIELD_MAX_WIDTH: i32 = 255;
pub const FIELD_MIN_HEIGHT: i32 = 3;
pub const FIELD_MAX_HEIGHT: i32 = 255;

/// Level map data transfer object.
#[derive(Clone, Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FieldDto {
	pub width: i32,
	pub height: i32,
	pub data: Vec<u8>,
	pub legend: Vec<Terrain>,
}

/// Camera entity focus definition.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CameraFocusTrigger {
	/// When the Player entity is at this position, the camera will focus on the target entity.
	pub player_pos: Vec2i,
	/// Index of the target entity.
	pub entity_index: usize,
	/// Kind of the target entity.
	///
	/// When the target entity is not of this kind, the camera will not focus on it.
	pub entity_kind: EntityKind,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Trophy {
	Author,
	Gold,
	Silver,
	Bronze,
}

impl Trophy {
	#[inline]
	pub fn to_str(self) -> &'static str {
		match self {
			Trophy::Author => "Author",
			Trophy::Gold => "Gold",
			Trophy::Silver => "Silver",
			Trophy::Bronze => "Bronze",
		}
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TrophyValues {
	pub author: i32,
	pub gold: i32,
	pub silver: i32,
	pub bronze: i32,
}

#[derive(Clone, Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Trophies {
	pub ticks: TrophyValues,
	pub steps: TrophyValues,
}

/// Level data transfer object.
#[derive(Clone, Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct LevelDto {
	pub name: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub author: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub hint: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub password: Option<String>,
	pub required_chips: i32,
	#[serde(default, skip_serializing_if = "is_default")]
	pub time_limit: i32,
	pub map: FieldDto,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub entities: Vec<EntityArgs>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub connections: Vec<FieldConn>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub camera_triggers: Vec<CameraFocusTrigger>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub replays: Option<Vec<ReplayDto>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub trophies: Option<Trophies>,
}

impl LevelDto {
	pub fn normalize(&mut self) {
		// Ensure required chips is non-negative
		if self.required_chips < 0 {
			self.required_chips = 0;
		}
		// Ensure time limit is non-negative
		if self.time_limit < 0 {
			self.time_limit = 0;
		}
		// Sort entities for consistent interaction order
		sort_entities(&mut self.entities);
	}
}
fn sort_entities(entities: &mut Vec<EntityArgs>) {
	entities.sort_by_key(|e| match e.kind {
		EntityKind::Player       => 1,
		EntityKind::PlayerNPC    => 1,
		EntityKind::Block        => 2,
		EntityKind::IceBlock     => 2,
		EntityKind::Chip         => 3,
		EntityKind::Flippers     => 3,
		EntityKind::FireBoots    => 3,
		EntityKind::IceSkates    => 3,
		EntityKind::SuctionBoots => 3,
		EntityKind::BlueKey      => 3,
		EntityKind::RedKey       => 3,
		EntityKind::GreenKey     => 3,
		EntityKind::YellowKey    => 3,
		EntityKind::Socket       => 4,
		EntityKind::Thief        => 4,
		EntityKind::Bomb         => 5,
		EntityKind::Bug          => 5,
		EntityKind::FireBall     => 5,
		EntityKind::PinkBall     => 5,
		EntityKind::Tank         => 5,
		EntityKind::Glider       => 5,
		EntityKind::Teeth        => 5,
		EntityKind::Walker       => 5,
		EntityKind::Blob         => 5,
		EntityKind::Paramecium   => 5,
	})
}

/// Connection between terrain tiles.
///
/// The connection defines the relationship between:
/// * A red button and associated clone machine terrain.
/// * A brown button and associated bear trap terrain.
/// * A teleport pad and destination terrain.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FieldConn {
	pub src: Vec2i,
	pub dest: Vec2i,
}

/// Level brush for editing levels.
#[derive(Clone, Debug)]
pub struct LevelBrush {
	pub width: i32,
	pub height: i32,
	pub terrain: Vec<Option<Terrain>>,
	pub entities: Vec<EntityArgs>,
	pub connections: Vec<FieldConn>,
}
impl LevelBrush {
	#[inline]
	pub fn is_pos_inside(&self, pos: Vec2i) -> bool {
		pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.height
	}
}
