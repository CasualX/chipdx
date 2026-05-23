use super::*;

cfg_select! {
	target_arch = "wasm32" => {
		use shade::shaders::glsl300es as shaders;
	}
	target_os = "android" => {
		use shade::shaders::glsl300es as shaders;
	}
	_ => {
		use shade::shaders::glsl330core as shaders;
	}
}

pub struct PostProcessCopy {
	pub quad: shade::d2::PostProcessQuad,
	pub shader: shade::ShaderProgram,
}

#[derive(Default)]
pub struct Resources {
	pub textures: HashMap<String, shade::Texture2D>,
	pub shaders: HashMap<String, shade::ShaderProgram>,

	pub effects: shade::Texture2D,
	pub spritesheet_texture: shade::Texture2D,
	pub spritesheet_meta: chipty::SpriteSheet<chipty::SpriteId>,

	pub shader: shade::ShaderProgram,
	pub shader_shadowmap: shade::ShaderProgram,
	pub shader2d_pixelart: shade::ShaderProgram,
	pub backbuffer_viewport: Bounds2i,

	pub colorshader: shade::ShaderProgram,
	pub uishader: shade::ShaderProgram,
	pub menubg: shade::Texture2D,
	pub menubg_scale: f32,

	pub font: shade::d2::FontResource<Option<shade::msdfgen::Font>>,

	pub backcolor: shade::Texture2D,
	pub backdepth: shade::Texture2D,
	pub viewport: Bounds2i,
	pub pp: Option<PostProcessCopy>,
	pub renderscale: f32,
}

#[track_caller]
fn load_png(
	resx: &mut Resources,
	g: &mut shade::Graphics,
	name: Option<&str>,
	fs: &crate::FileSystem,
	path: &str,
	props: &shade::TextureProps,
) -> Result<shade::Texture2D, shade::image::LoadImageError> {
	let data = fs.read(path).expect("Failed to read PNG file");
	let image = shade::image::DecodedImage::load_memory_png(data.as_slice())?;
	let tex = g.image(&(&image, props));
	if let Some(name) = name {
		resx.textures.insert(name.to_string(), tex);
	}
	Ok(tex)
}

impl Resources {
	pub fn load(&mut self, fs: &crate::FileSystem, config: &crate::config::Config, g: &mut shade::Graphics) {
		let resx = self;
		for (name, shader) in &config.shaders {
			let vs = fs.read_to_string(&shader.vertex_shader).expect("Failed to read shader vertex file");
			let fs = fs.read_to_string(&shader.fragment_shader).expect("Failed to read shader fragment file");
			let shader = g.shader_compile(&vs, &fs);
			resx.shaders.insert(name.to_string(), shader);
		}
		for (name, texture) in &config.textures {
			load_png(resx, g, Some(name.as_str()), fs, &texture.path, &texture.props).expect("Failed to load texture");
		}
		let shader = g.shader_compile(shaders::MTSDF_VS, shaders::MTSDF_FS);
		for (name, font_config) in &config.fonts {
			let font = fs.read_to_string(&font_config.meta).expect("Failed to read font meta file");
			let font: shade::msdfgen::FontDto = serde_json::from_str(&font).expect("Failed to parse font meta file");
			let font: Option<shade::msdfgen::Font> = Some(font.into());
			let data = fs.read(&font_config.atlas).expect("Failed to read font atlas file");
			let image = shade::image::DecodedImage::load_memory_png(data.as_slice()).expect("Failed to decode font atlas PNG");
			let image = image.to_rgba().map_colors(|[r, g, b, a]| shade::color::Rgba8 { r, g, b, a });
			let props = shade::TextureProps {
				mip_levels: 1,
				usage: shade::TextureUsage::TEXTURE,
				filter_min: shade::TextureFilter::Linear,
				filter_mag: shade::TextureFilter::Linear,
				wrap_u: shade::TextureWrap::Edge,
				wrap_v: shade::TextureWrap::Edge,
				..Default::default()
			};
			let texture = g.image(&(&image, &props));
			resx.textures.insert(name.to_string(), texture);
			// let texture = load_png(g, Some(name.as_str()), fs, &font_config.atlas, &).expect("Failed to load font atlas");
			let font = shade::d2::FontResource { font, shader, texture };
			resx.font = font;
		}

		resx.effects = resx.textures.get("Effects").unwrap().clone();
		resx.spritesheet_texture = resx.textures.get("SpriteSheet").unwrap().clone();
		let spritesheet_meta = fs.read_to_string("spritesheet.json").expect("Failed to read spritesheet metadata");
		resx.spritesheet_meta = serde_json::from_str(&spritesheet_meta).expect("Failed to parse spritesheet metadata");

		resx.shader = resx.shaders.get("PixelArt").unwrap().clone();
		resx.shader_shadowmap = resx.shaders.get("PixelArtShadowMap").unwrap().clone();
		resx.shader2d_pixelart = g.shader_compile(shaders::PIXELART_VS, shaders::PIXELART_FS);
		resx.colorshader = resx.shaders.get("Color").unwrap().clone();
		resx.uishader = resx.shaders.get("UI").unwrap().clone();
		resx.menubg = resx.textures.get("MenuBG").unwrap().clone();
		resx.menubg_scale = 2.0 * config.render_scale;
		if resx.pp.is_none() {
			resx.pp = Some(PostProcessCopy {
				quad: shade::d2::PostProcessQuad::create(g),
				shader: g.shader_compile(shaders::POST_PROCESS_VS, shaders::POST_PROCESS_PIXELART_FS),
			});
		}
		resx.renderscale = config.render_scale;
	}

	pub fn update_back(&mut self, g: &mut shade::Graphics) {
		let width = (self.backbuffer_viewport.width() as f32 * self.renderscale) as i32;
		let height = (self.backbuffer_viewport.height() as f32 * self.renderscale) as i32;
		self.viewport = Bounds2!(0, 0, width, height);
		self.backcolor = g.texture2d_update(self.backcolor, &shade::Texture2DInfo {
			width,
			height,
			format: shade::TextureFormat::SRGBA8,
			props: shade::TextureProps {
				mip_levels: 1,
				usage: shade::TextureUsage!(SAMPLED | COLOR_TARGET),
				filter_min: shade::TextureFilter::Linear,
				filter_mag: shade::TextureFilter::Linear,
				wrap_u: shade::TextureWrap::Edge,
				wrap_v: shade::TextureWrap::Edge,
				..Default::default()
			}
		});
		self.backdepth = g.texture2d_update(self.backdepth, &shade::Texture2DInfo {
			width,
			height,
			format: shade::TextureFormat::Depth24,
			props: shade::TextureProps {
				mip_levels: 1,
				usage: shade::TextureUsage!(SAMPLED | DEPTH_STENCIL_TARGET),
				filter_min: shade::TextureFilter::Nearest,
				filter_mag: shade::TextureFilter::Nearest,
				wrap_u: shade::TextureWrap::Edge,
				wrap_v: shade::TextureWrap::Edge,
				..Default::default()
			}
		});
	}

	pub fn present(&self, g: &mut shade::Graphics) {
		g.begin(&shade::BeginArgs::BackBuffer {
			viewport: self.backbuffer_viewport,
		});
		if let Some(pp) = &self.pp {
			pp.quad.draw(g, pp.shader, shade::BlendMode::Solid, &[
				&shade::UniformFn(|set| {
					set.value("u_texture", &self.backcolor);
				})
			]);
		}
		g.end();
	}
}
