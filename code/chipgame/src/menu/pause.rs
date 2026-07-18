use super::*;

#[derive(Default)]
pub struct PauseMenu {
	pub selected: u8,
	pub assist_mode: bool,
	pub has_warp: bool,
	pub level_number: i32,
	pub level_name: String,
}

const MENU_WITH_WARPS: MenuItems = MenuItems {
	labels: &[&"Resume", &"Set Warp Point", &"Warp Back", &"Restart", &"Options", &"Main Menu"],
	events: &[MenuEvent::ResumePlay, MenuEvent::SaveState, MenuEvent::LoadState, MenuEvent::RestartLevel, MenuEvent::OpenOptions, MenuEvent::OpenMainMenu],
};
const MENU_WITH_WARP_POINT: MenuItems = MenuItems {
	labels: &[&"Resume", &"Set Warp Point", &"Restart", &"Options", &"Main Menu"],
	events: &[MenuEvent::ResumePlay, MenuEvent::SaveState, MenuEvent::RestartLevel, MenuEvent::OpenOptions, MenuEvent::OpenMainMenu],
};
const MENU_WO_WARPS: MenuItems = MenuItems {
	labels: &[&"Resume", &"Restart", &"Options", &"Main Menu"],
	events: &[MenuEvent::ResumePlay, MenuEvent::RestartLevel, MenuEvent::OpenOptions, MenuEvent::OpenMainMenu],
};

impl PauseMenu {
	fn get_menu_items(&self) -> MenuItems<'static> {
		if !self.assist_mode {
			MENU_WO_WARPS
		}
		else if self.has_warp {
			MENU_WITH_WARPS
		}
		else {
			MENU_WITH_WARP_POINT
		}
	}
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		let items = self.get_menu_items();
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
		if input.b.is_pressed() || input.start.is_pressed() {
			events.push(MenuEvent::ResumePlay);
		}
		if input.select.is_pressed() {
			events.push(MenuEvent::CloseMenu);
			events.push(MenuEvent::OpenScoutMode);
		}
	}
	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &Resources) {
		let mut buf = shade::d2::TextBuffer::new();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let [top, bottom] = draw::flexv(rect, None, layout::Justify::Start, &[layout::Unit::Fr(1.0), layout::Unit::Fr(2.0)]);

		draw::DrawPlayTitle {
			level_number: self.level_number,
			level_name: &self.level_name,
			subtitle: Some(&"\x1b[color=#0f0]Paused!"),
		}.draw(&mut buf, &top, resx);

		let items = self.get_menu_items();

		draw::DrawMenuItems {
			items_text: items.labels,
			selected_index: self.selected as usize,
		}.draw(&mut buf, &bottom, resx);

		buf.draw(g);
	}
}
