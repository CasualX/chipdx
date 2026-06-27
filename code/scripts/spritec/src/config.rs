use std::{fs, path};

#[derive(Copy, Clone, Debug)]
pub enum GutterMode {
	ClampToEdge,
	SelfTiled,
	Transparent,
}
impl Into<shade::image::BlitGutterMode<[u8; 4]>> for GutterMode {
	fn into(self) -> shade::image::BlitGutterMode<[u8; 4]> {
		match self {
			GutterMode::ClampToEdge => shade::image::BlitGutterMode::Edge,
			GutterMode::SelfTiled => shade::image::BlitGutterMode::Repeat,
			GutterMode::Transparent => shade::image::BlitGutterMode::Border([0; 4]),
		}
	}
}

pub struct Sprite {
	pub name: String,
	pub frames: Vec<String>,
	pub transform: shade::atlas::Transform,
	pub gutter: GutterMode,
}

pub struct File {
	pub sprites: Vec<Sprite>,
}

impl File {
	pub fn load(root: &path::Path) -> File {
		let sprites = load_sprites(root);
		File { sprites }
	}
}

fn load_sprites(root: &path::Path) -> Vec<Sprite> {
	let config_path = root.join("_Sprites.ini");
	let config_text = fs::read_to_string(&config_path)
		.unwrap_or_else(|err| panic!("read sprite config {}: {}", config_path.display(), err));

	let mut sections: Vec<Sprite> = Vec::new();
	let mut current_name: Option<String> = None;
	let mut current_frames: Vec<String> = Vec::new();
	let mut current_transform = shade::atlas::Transform::None;
	let mut current_gutter = GutterMode::ClampToEdge;

	for item in ini_core::Parser::new(&config_text) {
		match item {
			ini_core::Item::Section(name) => {
				if let Some(prev_name) = current_name.take() {
					if current_frames.is_empty() {
						panic!("sprite {} missing Path entries", prev_name);
					}
					sections.push(Sprite { name: prev_name, frames: current_frames, transform: current_transform, gutter: current_gutter });
				}
				current_name = Some(name.to_string());
				current_transform = shade::atlas::Transform::None;
				current_frames = Vec::new();
			}
			ini_core::Item::Property(key, Some(value)) => {
				if key == "Frame" {
					current_frames.push(value.to_string());
				}
				else if key == "Transform" {
					current_transform = match value {
						"None" => shade::atlas::Transform::None,
						"FlipX" => shade::atlas::Transform::FlipX,
						"FlipY" => shade::atlas::Transform::FlipY,
						"FlipXY" => shade::atlas::Transform::Rotate180,
						"Rotate90" => shade::atlas::Transform::Rotate90,
						"Rotate180" => shade::atlas::Transform::Rotate180,
						"Rotate270" => shade::atlas::Transform::Rotate270,
						_ => panic!("unknown Transform value: {}", value),
					};
				}
				else if key == "Gutter" {
					current_gutter = match value {
						"ClampToEdge" => GutterMode::ClampToEdge,
						"SelfTiled" => GutterMode::SelfTiled,
						"Transparent" => GutterMode::Transparent,
						_ => panic!("unknown Gutter value: {}", value),
					};
				}
			}
			_ => {}
		}
	}

	if let Some(prev_name) = current_name.take() {
		if current_frames.is_empty() {
			panic!("sprite {} missing Path entries", prev_name);
		}
		sections.push(Sprite { name: prev_name, frames: std::mem::take(&mut current_frames), transform: current_transform, gutter: current_gutter });
	}

	let mut sprites: Vec<Sprite> = sections;

	sprites.sort_by(|a, b| a.name.cmp(&b.name));
	sprites
}
