use super::*;

#[derive(Default)]
pub struct UnlockLevelMenu {
	pub selected: i8,
	pub password: [Option<char>; 4],
}

const fn lookup(letter: u8) -> usize {
	let mut i = 0;
	while i < LETTERS_MAP.len() {
		if LETTERS_MAP[i] == letter {
			return i;
		}
		i += 1;
	}
	unreachable!()
}

macro_rules! create_map {
	($($src:ident => $dest:ident,)*) => {
		{
			let mut map = [-1; 26];
			$(map[lookup(stringify!($src).as_bytes()[0])] = lookup(stringify!($dest).as_bytes()[0]) as i8;)*
			map
		}
	}
}

// Maps querty positions to the letter down below
const LETTERS_MAP: &[u8; 26] = b"QWERTYUIOPASDFGHJKLZXCVBNM";

static PRESS_DOWN_MAP: [i8; 26] = create_map! {
	Q=>A, W=>S, E=>D, R=>F, T=>G, Y=>H, U=>J, I=>K, O=>L, P=>L,
	A=>Z, S=>Z, D=>X, F=>C, G=>V, H=>B, J=>N, K=>M, L=>M,
};
static PRESS_UP_MAP: [i8; 26] = create_map! {
	A=>Q, S=>W, D=>E, F=>R, G=>T, H=>Y, J=>U, K=>I, L=>O,
	Z=>S, X=>D, C=>F, V=>G, B=>H, N=>J, M=>K,
};
static PRESS_LEFT_MAP: [i8; 26] = create_map! {
	Q=>P, W=>Q, E=>W, R=>E, T=>R, Y=>T, U=>Y, I=>U, O=>I, P=>O,
	A=>L, S=>A, D=>S, F=>D, G=>F, H=>G, J=>H, K=>J, L=>K,
	Z=>M, X=>Z, C=>X, V=>C, B=>V, N=>B, M=>N,
};
static PRESS_RIGHT_MAP: [i8; 26] = create_map! {
	Q=>W, W=>E, E=>R, R=>T, T=>Y, Y=>U, U=>I, I=>O, O=>P, P=>Q,
	A=>S, S=>D, D=>F, F=>G, G=>H, H=>J, J=>K, K=>L, L=>A,
	Z=>X, X=>C, C=>V, V=>B, B=>N, N=>M, M=>Z,
};

impl UnlockLevelMenu {
	pub fn think(&mut self, input: &Input, events: &mut Vec<MenuEvent>) {
		if input.up.is_pressed() {
			let new_index = PRESS_UP_MAP[self.selected as usize];
			if new_index >= 0 {
				self.selected = new_index;
			}
		}

		if input.down.is_pressed() {
			let new_index = PRESS_DOWN_MAP[self.selected as usize];
			if new_index >= 0 {
				self.selected = new_index;
			}
		}

		if input.left.is_pressed() {
			let new_index = PRESS_LEFT_MAP[self.selected as usize];
			if new_index >= 0 {
				self.selected = new_index;
			}
		}

		if input.right.is_pressed() {
			let new_index = PRESS_RIGHT_MAP[self.selected as usize];
			if new_index >= 0 {
				self.selected = new_index;
			}
		}

		if input.a.is_pressed() {
			if let Some(slot) = self.password.iter().position(|&x| x.is_none()) {
				self.password[slot] = Some(LETTERS_MAP[self.selected as usize] as char);
			}
		}

		if input.b.is_pressed() {
			if let Some(slot) = self.password.iter().rposition(|&x| x.is_some()) {
				self.password[slot] = None;
			}
			else {
				events.push(MenuEvent::CloseMenu);
			}
		}

		if input.start.is_pressed() {
			if let [Some(_1), Some(_2), Some(_3), Some(_4)] = self.password {
				events.push(MenuEvent::EnterPassword { code: [_1 as u8, _2 as u8, _3 as u8, _4 as u8] });
			}
		}
	}
	pub fn draw(&mut self, g: &mut dyn shade::IGraphics, resx: &Resources) {
		let mut buf = shade::d2::TextBuffer::new();
		buf.blend_mode = shade::BlendMode::Alpha;
		buf.shader = Some(&*resx.font.shader);

		let rect = resx.viewport.cast();
		buf.uniform.transform = Transform2f::ortho(rect);
		buf.uniform.texture = &*resx.font.texture;

		let size = resx.font_size();

		let scribe = shade::d2::Scribe {
			font_size: size,
			line_height: size * (5.0 / 4.0),
			color: Vec4(255, 255, 255, 255),
			..Default::default()
		};

		let rect = Bounds2!(0.0, size + size, resx.viewport.width() as f32, size + size);

		buf.text_lines(&resx.font, &scribe, &rect, shade::d2::TextAlign::TopCenter, &[
			&format_args!("Enter Password: {} {} {} {}", self.password[0].unwrap_or('_'), self.password[1].unwrap_or('_'), self.password[2].unwrap_or('_'), self.password[3].unwrap_or('_')),
		]);

		let letters = &[&LETTERS_MAP[0..10], &LETTERS_MAP[10..19], &LETTERS_MAP[19..26]];
		let height = letters.len() as f32 * size * 1.5;
		for (i, &line) in letters.iter().enumerate() {
			let y = (resx.viewport.height() as f32 - height) * 0.5 + i as f32 * size * 1.5;
			let width = line.len() as f32 * size * 1.5;
			for (j, &chr) in line.iter().enumerate() {
				let xstart = (resx.viewport.width() as f32 - width) * 0.5;
				let current_index = match i { 0 => j as i8, 1 => j as i8 + 10, 2 => j as i8 + 19, _ => unreachable!() };
				let rect = Bounds2!(xstart + j as f32 * size * 1.5, y, xstart + j as f32 * size * 1.5, y);
				let scribe = shade::d2::Scribe {
					font_size: size,
					line_height: size * (5.0 / 4.0),
					color: if current_index == self.selected { Vec4(255, 255, 255, 255) } else { Vec4(128, 128, 128, 255) },
					..Default::default()
				};
				let chr = chr as char;
				buf.text_lines(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleCenter, &[&chr]);
			}
		}

		buf.draw(g);
	}
}
