//! Configuration parsing.

use std::collections::HashMap;

#[derive(Default)]
pub struct ShaderConfig {
	pub shader: String,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum PostProcess {
	#[default]
	None,
	Crt,
}

pub struct TextureConfig {
	pub path: String,
	pub props: shade::TextureProps,
}
impl Default for TextureConfig {
	fn default() -> Self {
		Self {
			path: String::new(),
			props: shade::TextureProps {
				mip_levels: 1,
				usage: shade::TextureUsage::TEXTURE,
				filter_min: shade::TextureFilter::Linear,
				filter_mag: shade::TextureFilter::Linear,
				wrap_u: shade::TextureWrap::Repeat,
				wrap_v: shade::TextureWrap::Repeat,
				..Default::default()
			}
		}
	}
}

#[derive(Default)]
pub struct FontConfig {
	pub atlas: String,
	pub meta: String,
	pub shader: String,
}

pub struct Config {
	pub vsync: bool,
	pub multisampling: u8,
	pub render_scale: f32,
	pub post_process: PostProcess,
	pub sound_fx: HashMap<chipty::SoundFx, String>,
	pub music: HashMap<chipty::MusicId, String>,
	pub shaders: HashMap<String, ShaderConfig>,
	pub textures: HashMap<String, TextureConfig>,
	pub fonts: HashMap<String, FontConfig>,
}

impl Config {
	pub fn parse(text: &str) -> Config {
		enum Section<'a> {
			Error,
			General,
			SoundFx,
			Music,
			Shader(&'a str),
			Texture(&'a str),
			Font(&'a str),
		}

		let mut section = Section::General;
		let mut vsync = false;
		let mut multisampling = 0;
		let mut render_scale = 1.0f32;
		let mut post_process = PostProcess::None;
		let mut sound_fx: HashMap<chipty::SoundFx, String> = HashMap::new();
		let mut music: HashMap<chipty::MusicId, String> = HashMap::new();
		let mut shaders: HashMap<String, ShaderConfig> = HashMap::new();
		let mut textures: HashMap<String, TextureConfig> = HashMap::new();
		let mut fonts: HashMap<String, FontConfig> = HashMap::new();

		for item in ini_core::Parser::new(text) {
			match item {
				ini_core::Item::Property(key, Some(value)) => match section {
					Section::Error => (),
					Section::General => match key {
						"VSync" => { if let Ok(v) = value.parse::<bool>() { vsync = v; } }
						"Multisampling" => { if let Ok(v) = value.parse::<u8>() { multisampling = v; } }
						"RenderScale" => { if let Ok(v) = value.parse::<f32>() { render_scale = v; } }
						"PostProcess" => {
							post_process = match value {
								"CRT" => PostProcess::Crt,
								"None" | "" => PostProcess::None,
								_ => PostProcess::None,
							};
						}
						_ => {}
					},
					Section::SoundFx => {
						if let Ok(fx) = key.parse::<chipty::SoundFx>() {
							sound_fx.insert(fx, value.to_string());
						}
					}
					Section::Music => {
						if let Ok(id) = key.parse::<chipty::MusicId>() {
							music.insert(id, value.to_string());
						}
					}
					Section::Shader(name) => {
						let entry = shaders.entry(name.to_string()).or_default();
						match key {
							"Shader" => entry.shader = value.to_string(),
							_ => {}
						}
					}
					Section::Texture(name) => {
						let entry = textures.entry(name.to_string()).or_default();
						match key {
							"Path" => entry.path = value.to_string(),
							"Filter" => {
								let filter = match value {
									"Nearest" => shade::TextureFilter::Nearest,
									"Linear" => shade::TextureFilter::Linear,
									_ => panic!("Unknown texture filter mode: {}", value),
								};
								entry.props.filter_min = filter;
								entry.props.filter_mag = filter;
							}
							"Wrap" => {
								let wrap = match value {
									"Edge" => shade::TextureWrap::Edge,
									"Border" => shade::TextureWrap::Border,
									"Repeat" => shade::TextureWrap::Repeat,
									"Mirror" => shade::TextureWrap::Mirror,
									_ => panic!("Unknown texture wrap mode: {}", value),
								};
								entry.props.wrap_u = wrap;
								entry.props.wrap_v = wrap;
							}
							_ => {}
						}
					}
					Section::Font(name) => {
						let entry = fonts.entry(name.to_string()).or_default();
						match key {
							"Atlas" => entry.atlas = value.to_string(),
							"Meta" => entry.meta = value.to_string(),
							"Shader" => entry.shader = value.to_string(),
							_ => {}
						}
					}
				},
				ini_core::Item::Section(name) => {
					section = match name {
						"SoundFx" => Section::SoundFx,
						"Music" => Section::Music,
						_ => {
							if let Some(name) = name.strip_prefix("Shader.") {
								Section::Shader(name)
							}
							else if let Some(name) = name.strip_prefix("Texture.") {
								Section::Texture(name)
							}
							else if let Some(name) = name.strip_prefix("Font.") {
								Section::Font(name)
							}
							else {
								Section::Error
							}
						},
					};
				}
				_ => {}
			}
		}

		Config {
			multisampling,
			vsync,
			post_process,
			sound_fx,
			music,
			shaders,
			textures,
			fonts,
			render_scale,
		}
	}
}
