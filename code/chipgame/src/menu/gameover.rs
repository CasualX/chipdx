use super::*;

#[derive(Default)]
pub struct GameOverMenu {
	pub selected: u8,
	pub reason: Option<chipcore::GameOverReason>,
	pub has_warp: bool,
	pub level_number: i32,
	pub level_name: String,
	pub attempts: i32,
	pub time: i32,
	pub steps: i32,
	pub bonks: i32,
}

const MENU_WITH_WARPS: MenuItems = MenuItems {
	labels: &[&"Warp Back", &"Start Over", &"Main Menu"],
	events: &[MenuEvent::LoadState, MenuEvent::RetryLevel, MenuEvent::OpenMainMenu],
};
const MENU_WO_WARPS: MenuItems = MenuItems {
	labels: &[&"Start Over", &"Main Menu"],
	events: &[MenuEvent::RetryLevel, MenuEvent::OpenMainMenu],
};

impl GameOverMenu {
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		let items = if self.has_warp { MENU_WITH_WARPS } else { MENU_WO_WARPS };
		if input.up.is_pressed() {
			if self.selected > 0 {
				self.selected -= 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.down.is_pressed() {
			if self.selected < items.events.len() as u8 - 1 {
				self.selected += 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.a.is_pressed() {
			if let Some(&event) = items.events.get(self.selected as usize) {
				events.push(event);
			}
			events.push(MenuEvent::CursorSelect);
		}
	}
	pub fn draw(&mut self, g: &mut shade::Graphics, resx: &Resources) {
		let mut buf = shade::d2::TextBuffer::new();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let [top, middle, bottom] = draw::flexv(rect, None, layout::Justify::Start, &[layout::Unit::Fr(1.0); 3]);

		let reason = {
			use chipcore::GameOverReason::*;
			match self.reason {
				Some(LevelComplete) => "Level Complete!",
				Some(Drowned) => "Ooops! Chip can't swim without flippers!",
				Some(Burned) => "Ooops! Don't step in the fire without fire boots!",
				Some(Bombed) => "Ooops! Don't touch the bombs!",
				Some(TimeOut) => "Ooops! Out of time!",
				Some(Collided) => "Ooops! Watch out for moving blocks!",
				Some(Eaten) => "Ooops! Look out for creatures!",
				Some(NotOkay) => "Ooops! You're not okay!",
				None => "Game over!",
			}
		};

		draw::DrawPlayTitle {
			level_number: self.level_number,
			level_name: &self.level_name,
			subtitle: Some(&fmtools::fmt!("\x1b[color=#f08]"{reason})),
		}.draw(&mut buf, &top, resx);

		let [_, middle, _] = draw::flexh(middle, None, layout::Justify::Center, &[layout::Unit::Fr(1.0); 3]);

		draw::DrawScoreCard {
			attempts: self.attempts,
			time: self.time,
			steps: self.steps,
			bonks: self.bonks,
			time_high_score: -2,
			steps_high_score: -2,
		}.draw(&mut buf, &middle, resx);

		let items = if self.has_warp { MENU_WITH_WARPS } else { MENU_WO_WARPS };

		draw::DrawMenuItems {
			items_text: items.labels,
			selected_index: self.selected as usize,
		}.draw(&mut buf, &bottom, resx);

		buf.draw(g);
	}
}
