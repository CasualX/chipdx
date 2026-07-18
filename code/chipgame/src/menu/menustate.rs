use super::*;

pub enum Menu {
	LevelSet(levelset::LevelSetMenu),
	Main(MainMenu),
	GameWin(GameWinMenu),
	GameOver(GameOverMenu),
	Pause(PauseMenu),
	Options(OptionsMenu),
	GoToLevel(gotolevel::GoToLevel),
	UnlockLevel(unlocklevel::UnlockLevelMenu),
	About(AboutMenu),
	Scout(ScoutMode),
}
impl Menu {
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		match self {
			Menu::LevelSet(menu) => menu.think(input, events),
			Menu::Main(menu) => menu.think(input, events),
			Menu::GameWin(menu) => menu.think(input, events),
			Menu::GameOver(menu) => menu.think(input, events),
			Menu::Pause(menu) => menu.think(input, events),
			Menu::Options(menu) => menu.think(input, events),
			Menu::GoToLevel(menu) => menu.think(input, events),
			Menu::UnlockLevel(menu) => menu.think(input, events),
			Menu::About(menu) => menu.think(input, events),
			Menu::Scout(menu) => menu.think(input, events),
		}
	}
	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &Resources, time: f64) {
		match self {
			Menu::LevelSet(menu) => menu.draw(g, resx),
			Menu::Main(menu) => menu.draw(g, resx),
			Menu::GameWin(menu) => menu.draw(g, resx, time),
			Menu::GameOver(menu) => menu.draw(g, resx),
			Menu::Pause(menu) => menu.draw(g, resx),
			Menu::Options(menu) => menu.draw(g, resx),
			Menu::GoToLevel(menu) => menu.draw(g, resx),
			Menu::UnlockLevel(menu) => menu.draw(g, resx),
			Menu::About(menu) => menu.draw(g, resx),
			Menu::Scout(menu) => menu.draw(g, resx),
		}
	}
}

#[derive(Default)]
pub struct MenuState {
	pub stack: Vec<Menu>,
	pub events: Vec<MenuEvent>,
	pub input: chipcore::Input,
}

impl MenuState {
	pub fn think(&mut self, input: &chipcore::Input) -> bool {
		let menu_input = Input {
			a: KeyState::w(self.input.a, input.a),
			b: KeyState::w(self.input.b, input.b),
			up: KeyState::w(self.input.up, input.up),
			down: KeyState::w(self.input.down, input.down),
			left: KeyState::w(self.input.left, input.left),
			right: KeyState::w(self.input.right, input.right),
			start: KeyState::w(self.input.start, input.start),
			select: KeyState::w(self.input.select, input.select),
		};
		if let Some(menu) = self.stack.last_mut() {
			menu.think(&menu_input, &mut self.events);
		}
		self.input = *input;
		!self.stack.is_empty()
	}
	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &Resources, time: f64) {
		if let Some(menu) = self.stack.last_mut() {
			menu.draw(g, resx, time);
		}
	}
	pub fn close_all(&mut self) {
		self.stack.clear();
	}
	pub fn close_menu(&mut self, can_close_last_menu: bool) {
		if can_close_last_menu || self.stack.len() > 1 {
			let _ = self.stack.pop();
		}
	}
	pub fn open_main(&mut self, start_from_continue: bool, title: &str) {
		self.stack.clear();
		let menu = MainMenu {
			title: title.to_string(),
			selected: if start_from_continue { 1 } else { 0 },
		};
		self.stack.push(Menu::Main(menu));
	}
}
