use super::*;

#[derive(Default)]
pub struct OptionsMenu {
	pub selected: u8,
	pub options: chipty::OptionsDto,
}

impl OptionsMenu {
	const ITEMS_COUNT: u8 = 10;
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		if input.up.is_pressed() {
			if self.selected > 0 {
				self.selected = self.selected - 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.down.is_pressed() {
			if self.selected < Self::ITEMS_COUNT - 1 {
				self.selected = self.selected + 1;
				events.push(MenuEvent::CursorMove);
			}
		}
		if input.start.is_pressed() {
			events.push(MenuEvent::ResumePlay);
		}
		if input.a.is_pressed() {
			let evt = match self.selected {
				0 => {
					self.options.background_music = !self.options.background_music;
					MenuEvent::SetBackgroundMusic { value: self.options.background_music }
				}
				1 => {
					self.options.sound_effects = !self.options.sound_effects;
					MenuEvent::SetSoundEffects { value: self.options.sound_effects }
				}
				2 => {
					self.options.perspective = !self.options.perspective;
					MenuEvent::SetPerspective { value: self.options.perspective }
				}
				3 => {
					self.options.zoom_mode = self.options.zoom_mode.cycle();
					MenuEvent::SetZoomMode { value: self.options.zoom_mode }
				}
				4 => {
					self.options.assist_mode = !self.options.assist_mode;
					MenuEvent::SetAssistMode { value: self.options.assist_mode }
				}
				5 => {
					self.options.step_mode = !self.options.step_mode;
					MenuEvent::SetStepMode { value: self.options.step_mode }
				}
				6 => {
					self.options.auto_save_replay = !self.options.auto_save_replay;
					MenuEvent::SetAutoSaveReplay { value: self.options.auto_save_replay }
				}
				7 => {
					self.options.speedrun_mode = !self.options.speedrun_mode;
					MenuEvent::SetSpeedrunMode { value: self.options.speedrun_mode }
				}
				8 => {
					self.options.developer_mode = !self.options.developer_mode;
					MenuEvent::SetDeveloperMode { value: self.options.developer_mode }
				}
				_ => MenuEvent::CloseMenu,
			};
			events.push(evt);
			events.push(MenuEvent::CursorSelect);
		}
		if input.b.is_pressed() {
			events.push(MenuEvent::CloseMenu);
		}
	}
	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &Resources) {
		let mut buf = shade::d2::TextBuffer::new();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let get_flag = |state| if state { "\x1b[color=#0f0]ON" } else { "\x1b[color=#f00]OFF" };
		let get_zoom = |mode| match mode {
			chipty::ZoomMode::Wide => "\x1b[color=#ff0]Wide",
			chipty::ZoomMode::Classic => "\x1b[color=#0f0]Classic",
			chipty::ZoomMode::Fit => "\x1b[color=#0ff]Fit",
			chipty::ZoomMode::Editor => "\x1b[color=#0ff]Editor",
		};

		let items: [&dyn fmt::Display; _] = [
			&fmtools::fmt!("Background music: "{get_flag(self.options.background_music)}),
			&fmtools::fmt!("Sound effects: "{get_flag(self.options.sound_effects)}),
			&fmtools::fmt!("Perspective: "{get_flag(self.options.perspective)}),
			&fmtools::fmt!("Camera zoom: "{get_zoom(self.options.zoom_mode)}),
			&fmtools::fmt!("Assist mode: "{get_flag(self.options.assist_mode)}),
			&fmtools::fmt!("Step mode: "{get_flag(self.options.step_mode)}),
			&fmtools::fmt!("Auto save replays: "{get_flag(self.options.auto_save_replay)}),
			&fmtools::fmt!("Speedrun mode: "{get_flag(self.options.speedrun_mode)}),
			&fmtools::fmt!("Developer mode: "{get_flag(self.options.developer_mode)}),
			&"Back",
		];

		draw::DrawMenuItems {
			items_text: &items,
			selected_index: self.selected as usize,
		}.draw(&mut buf, &rect, resx);

		buf.draw(g);
	}
}
