use super::*;

pub struct EditorPlayState {
	pub level: String,
	pub fx: Box<fx::FxState>,
	pub input: Input,
	pub screen_size: Vec2i,
}

impl EditorPlayState {
	pub fn key_left(&mut self, pressed: bool) {
		self.input.key_left = pressed;
	}
	pub fn key_right(&mut self, pressed: bool) {
		self.input.key_right = pressed;
	}
	pub fn key_up(&mut self, pressed: bool) {
		self.input.key_up = pressed;
	}
	pub fn key_down(&mut self, pressed: bool) {
		self.input.key_down = pressed;
	}
	pub fn set_screen_size(&mut self, width: i32, height: i32) {
		self.screen_size = Vec2::new(width, height);
	}
	pub fn think(&mut self) {
		let input = chipcore::Input {
			a: false,
			b: false,
			up: self.input.key_up,
			down: self.input.key_down,
			left: self.input.key_left,
			right: self.input.key_right,
			start: false,
			select: false,
		};
		self.fx.think(&input, false);
	}
	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &fx::Resources, time: f64) {
		render::drawbg(g, resx);
		self.fx.draw(g, resx, time);
	}
	pub fn save_replay(&mut self) {
		let replay_dto: chipty::ReplayDto = self.fx.game.save_replay(self.fx.game_realtime);
		let mut level_dto: chipty::LevelDto = serde_json::from_str(&self.level).unwrap();
		level_dto.replays = Some(vec![replay_dto]);
		self.level = serde_json::to_string(&level_dto).unwrap();
	}
	pub fn play_stats(&self) -> EditorPlayStats {
		EditorPlayStats {
			realtime: self.fx.game_realtime,
			ticks: self.fx.game.time,
			steps: self.fx.game.ps.steps,
			bonks: self.fx.game.ps.bonks,
		}
	}
}
