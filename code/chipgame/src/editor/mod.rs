//! Level editor.

use std::{fmt, mem};
use cvmath::*;
use chipty::{EntityArgs, EntityKind, FieldConn, LevelDto, Terrain};

use crate::fx;
use crate::render;
use crate::menu;

mod editstate;
mod fxeditstate;
mod playstate;
mod tiles;
mod tool;
mod history;

use self::editstate::EditorEditState;
use self::fxeditstate::FxEditState;
use self::playstate::EditorPlayState;
pub use self::tool::*;
pub use self::history::History;

#[derive(Clone, Debug)]
pub struct EditorPlayStats {
	pub realtime: f32,
	pub ticks: i32,
	pub steps: i32,
	pub bonks: i32,
}

static TERRAIN_SAMPLES: [[Terrain; 2]; 27] = [
	[Terrain::Blank, Terrain::Floor],
	[Terrain::Dirt, Terrain::Gravel],
	[Terrain::Wall, Terrain::CloneMachine],
	[Terrain::HiddenWall, Terrain::InvisibleWall],
	[Terrain::RealBlueWall, Terrain::FakeBlueWall],
	[Terrain::BlueLock, Terrain::RedLock],
	[Terrain::GreenLock, Terrain::YellowLock],
	[Terrain::Exit, Terrain::Hint],
	[Terrain::Water, Terrain::Fire],
	[Terrain::WaterHazard, Terrain::DirtBlock],
	[Terrain::ThinWallE, Terrain::ThinWallS],
	[Terrain::ThinWallN, Terrain::ThinWallW],
	[Terrain::ThinWallNW, Terrain::ThinWallNE],
	[Terrain::ThinWallSW, Terrain::ThinWallSE],
	[Terrain::ThinWallH, Terrain::ThinWallV],
	[Terrain::Ice, Terrain::Ice],
	[Terrain::IceNW, Terrain::IceNE],
	[Terrain::IceSW, Terrain::IceSE],
	[Terrain::ToggleFloor, Terrain::ToggleWall],
	[Terrain::GreenButton, Terrain::RedButton],
	[Terrain::BrownButton, Terrain::BlueButton],
	[Terrain::BearTrap, Terrain::RecessedWall],
	[Terrain::Teleport, Terrain::ForceRandom],
	[Terrain::ForceE, Terrain::ForceS],
	[Terrain::ForceN, Terrain::ForceW],
	[Terrain::CloneBlockE, Terrain::CloneBlockS],
	[Terrain::CloneBlockN, Terrain::CloneBlockW],
];

static ENTITY_SAMPLES: [(EntityKind, chipty::SpriteId); 24] = [
	(EntityKind::Player, chipty::SpriteId::PlayerWalkIdle),
	(EntityKind::Chip, chipty::SpriteId::Chip),
	(EntityKind::Socket, chipty::SpriteId::Socket),
	(EntityKind::Block, chipty::SpriteId::DirtBlock),
	(EntityKind::IceBlock, chipty::SpriteId::IceBlock),
	(EntityKind::Flippers, chipty::SpriteId::Flippers),
	(EntityKind::FireBoots, chipty::SpriteId::FireBoots),
	(EntityKind::IceSkates, chipty::SpriteId::IceSkates),
	(EntityKind::SuctionBoots, chipty::SpriteId::SuctionBoots),
	(EntityKind::BlueKey, chipty::SpriteId::BlueKey),
	(EntityKind::RedKey, chipty::SpriteId::RedKey),
	(EntityKind::GreenKey, chipty::SpriteId::GreenKey),
	(EntityKind::YellowKey, chipty::SpriteId::YellowKey),
	(EntityKind::Thief, chipty::SpriteId::Thief),
	(EntityKind::Bomb, chipty::SpriteId::Bomb),
	(EntityKind::Bug, chipty::SpriteId::BugN),
	(EntityKind::FireBall, chipty::SpriteId::Fireball),
	(EntityKind::PinkBall, chipty::SpriteId::PinkBall),
	(EntityKind::Tank, chipty::SpriteId::TankN),
	(EntityKind::Glider, chipty::SpriteId::GliderN),
	(EntityKind::Teeth, chipty::SpriteId::TeethN),
	(EntityKind::Walker, chipty::SpriteId::WalkerN),
	(EntityKind::Blob, chipty::SpriteId::Blob),
	(EntityKind::Paramecium, chipty::SpriteId::ParameciumN),
];

#[derive(Default)]
pub struct Input {
	pub left_click: bool,
	pub right_click: bool,
	pub key_left: bool,
	pub key_right: bool,
	pub key_up: bool,
	pub key_down: bool,
	pub key_shift: bool,
}

pub enum EditorState {
	Edit(Box<EditorEditState>),
	Play(Box<EditorPlayState>),
}

impl EditorState {
	pub fn is_playing(&self) -> bool {
		matches!(self, EditorState::Play(_))
	}

	pub fn new(json: &str) -> EditorState {
		EditorState::Edit(Box::new(EditorEditState::load_level(json)))
	}
	pub fn reload_level(&mut self, json: &str) {
		match self {
			EditorState::Edit(s) => s.reload_level(json),
			EditorState::Play(_) => {}
		}
	}
	pub fn save_level(&self) -> String {
		match self {
			EditorState::Edit(s) => s.save_level(),
			EditorState::Play(s) => s.level.clone(),
		}
	}
	pub fn set_screen_size(&mut self, width: i32, height: i32) {
		match self {
			EditorState::Edit(s) => s.set_screen_size(width, height),
			EditorState::Play(s) => s.set_screen_size(width, height),
		}
	}
	pub fn mouse_move(&mut self, mouse_x: i32, mouse_y: i32) {
		match self {
			EditorState::Edit(s) => s.mouse_move(mouse_x, mouse_y),
			EditorState::Play(_) => (),
		}
	}
	pub fn key_left(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.key_left(pressed),
			EditorState::Play(s) => s.key_left(pressed),
		}
	}
	pub fn key_right(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.key_right(pressed),
			EditorState::Play(s) => s.key_right(pressed),
		}
	}
	pub fn key_up(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.key_up(pressed),
			EditorState::Play(s) => s.key_up(pressed),
		}
	}
	pub fn key_down(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.key_down(pressed),
			EditorState::Play(s) => s.key_down(pressed),
		}
	}
	pub fn key_shift(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.key_shift(pressed),
			EditorState::Play(_) => (),
		}
	}
	pub fn zoom_in(&mut self) {
		match self {
			EditorState::Edit(s) => s.zoom_in(),
			EditorState::Play(_) => {},
		}
	}
	pub fn zoom_out(&mut self) {
		match self {
			EditorState::Edit(s) => s.zoom_out(),
			EditorState::Play(_) => {},
		}
	}
	pub fn pan_view(&mut self, delta_x: f32, delta_y: f32) {
		match self {
			EditorState::Edit(s) => s.pan_view(Vec2f(delta_x, delta_y)),
			EditorState::Play(_) => {},
		}
	}
	pub fn toggle_play(&mut self) {
		match self {
			EditorState::Edit(s) => {
				let level = s.save_level();
				let level_dto = serde_json::from_str(&level).unwrap();
				let mut fx = fx::FxState::new(0, &level_dto, chipcore::RngSeed::System, crate::play::tile_gfx);
				fx.time = s.fx.time;
				let edit_view = s.fx.camera.save_state();

				fx.camera.set_perspective(true);
				fx.camera.load_state(edit_view, fx.time);
				*self = EditorState::Play(Box::new(EditorPlayState {
					level,
					fx,
					input: Input::default(),
					screen_size: s.screen_size,
				}));
			}
			EditorState::Play(s) => {
				let play_view = s.fx.camera.save_state();
				let mut state = EditorEditState::load_level(&s.level);
				state.screen_size = s.screen_size;
				state.fx.time = s.fx.time;

				state.fx.camera.load_state(play_view, state.fx.time);
				*self = EditorState::Edit(Box::new(state));
			},
		}
	}
	pub fn save_replay(&mut self) {
		if let EditorState::Play(s) = self {
			s.save_replay();
		}
	}
	pub fn think(&mut self) {
		match self {
			EditorState::Edit(s) => s.think(),
			EditorState::Play(s) => s.think(),
		}
	}
	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &fx::Resources, time: f64) {
		match self {
			EditorState::Edit(s) => s.draw(g, resx, time),
			EditorState::Play(s) => s.draw(g, resx, time),
		}
	}

	pub fn play_stats(&self) -> Option<EditorPlayStats> {
		match self {
			EditorState::Play(s) => Some(s.play_stats()),
			EditorState::Edit(_) => None,
		}
	}

	pub fn take_fx_events(&mut self) -> Vec<fx::FxEvent> {
		match self {
			EditorState::Edit(_) => Vec::new(),
			EditorState::Play(s) => mem::take(&mut s.fx.events),
		}
	}

	pub fn tool_terrain(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.tool_terrain(pressed),
			EditorState::Play(_) => {},
		}
	}
	pub fn tool_entity(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.tool_entity(pressed),
			EditorState::Play(_) => {},
		}
	}
	pub fn tool_connection(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.tool_connection(pressed),
			EditorState::Play(_) => {},
		}
	}
	pub fn tool_icepath(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.tool_icepath(pressed),
			EditorState::Play(_) => {},
		}
	}
	pub fn tool_forcepath(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.tool_forcepath(pressed),
			EditorState::Play(_) => {},
		}
	}
	pub fn tool_entorder(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.tool_entorder(pressed),
			EditorState::Play(_) => {},
		}
	}

	pub fn resize(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
		match self {
			EditorState::Edit(s) => s.resize(left, top, right, bottom),
			EditorState::Play(_) => {},
		}
	}

	pub fn expand_top(&mut self) {
		match self {
			EditorState::Edit(s) => s.resize(0, 1, 0, 0),
			EditorState::Play(_) => {},
		}
	}
	pub fn expand_bottom(&mut self) {
		match self {
			EditorState::Edit(s) => s.resize(0, 0, 0, 1),
			EditorState::Play(_) => {},
		}
	}
	pub fn expand_left(&mut self) {
		match self {
			EditorState::Edit(s) => s.resize(1, 0, 0, 0),
			EditorState::Play(_) => {},
		}
	}
	pub fn expand_right(&mut self) {
		match self {
			EditorState::Edit(s) => s.resize(0, 0, 1, 0),
			EditorState::Play(_) => {},
		}
	}
	pub fn crop_top(&mut self) {
		match self {
			EditorState::Edit(s) => s.resize(0, -1, 0, 0),
			EditorState::Play(_) => {},
		}
	}
	pub fn crop_bottom(&mut self) {
		match self {
			EditorState::Edit(s) => s.resize(0, 0, 0, -1),
			EditorState::Play(_) => {},
		}
	}
	pub fn crop_left(&mut self) {
		match self {
			EditorState::Edit(s) => s.resize(-1, 0, 0, 0),
			EditorState::Play(_) => {},
		}
	}
	pub fn crop_right(&mut self) {
		match self {
			EditorState::Edit(s) => s.resize(0, 0, -1, 0),
			EditorState::Play(_) => {},
		}
	}

	pub fn left_click(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.left_click(pressed),
			EditorState::Play(_) => {},
		}
	}
	pub fn cancel_left_click(&mut self) {
		match self {
			EditorState::Edit(s) => s.cancel_left_click(),
			EditorState::Play(_) => {},
		}
	}
	pub fn right_click(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.right_click(pressed),
			EditorState::Play(_) => {},
		}
	}
	pub fn delete(&mut self, pressed: bool) {
		match self {
			EditorState::Edit(s) => s.delete(pressed),
			EditorState::Play(_) => {},
		}
	}

	pub fn undo(&mut self) {
		match self {
			EditorState::Edit(s) => s.undo(),
			EditorState::Play(_) => {},
		}
	}
	pub fn redo(&mut self) {
		match self {
			EditorState::Edit(s) => s.redo(),
			EditorState::Play(_) => {},
		}
	}

	pub fn sample(&mut self) {
		match self {
			EditorState::Edit(s) => s.sample(),
			EditorState::Play(_) => {},
		}
	}

	pub fn get_tool(&self) -> Option<&ToolState> {
		match self {
			EditorState::Edit(s) => s.tool.as_ref(),
			EditorState::Play(_) => None,
		}
	}

	pub fn get_music(&self, music_enabled: bool) -> Option<chipty::MusicId> {
		if music_enabled {
			match self {
				EditorState::Edit(_) => Some(chipty::MusicId::MenuMusic),
				EditorState::Play(_) => Some(chipty::MusicId::GameMusic),
			}
		}
		else {
			None
		}
	}
}
