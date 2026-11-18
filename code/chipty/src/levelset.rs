use super::*;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum LevelRef {
	Indirect(String),
	Direct(LevelDto),
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LevelSetDto {
	pub name: String,
	pub title: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub subtitle: Option<String>,
	#[serde(default, skip_serializing_if = "is_default")]
	pub difficulty: i32,
	/// Makes every level available without completing earlier levels.
	#[serde(default, skip_serializing_if = "is_default")]
	pub unlock_all_levels: bool,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub about: Option<Vec<String>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub splash: Option<String>,
	pub levels: Vec<LevelRef>,
}
