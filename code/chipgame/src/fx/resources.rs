use super::*;

struct ShaderFsInterface<'a> {
	fs: &'a crate::FileSystem,
	base: &'a str,
}
impl shade::IShaderInterface for ShaderFsInterface<'_> {
	fn include_source(&mut self, name: &str) -> std::io::Result<String> {
		if name.contains("..") {
			return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
		}
		let path = if self.base.is_empty() {
			name.to_string()
		}
		else {
			format!("{}/{}", self.base, name)
		};
		self.fs.read_to_string(&path)
	}
}

fn split_shader_path(path: &str) -> (&str, &str) {
	path.rsplit_once('/').unwrap_or(("", path))
}

struct StaticShaderInterface {
	name: &'static str,
	source: &'static str,
}
impl shade::IShaderInterface for StaticShaderInterface {
	fn include_source(&mut self, name: &str) -> std::io::Result<String> {
		if name == self.name {
			Ok(self.source.to_string())
		}
		else {
			Err(std::io::Error::from(std::io::ErrorKind::NotFound))
		}
	}
}

fn compile_static_shader(g: &mut shade::Graphics, name: &'static str, source: &'static str) -> Box<dyn shade::ShaderProgram> {
	let mut interface = StaticShaderInterface { name, source };
	g.shader_compile(&mut interface, name, &[])
}

pub struct PostProcessEffect {
	pub quad: shade::d2::PostProcessQuad,
	pub shader_copy: Box<dyn shade::ShaderProgram>,
	pub shader_crt: Box<dyn shade::ShaderProgram>,
}

pub struct Resources {
	pub effects: Box<dyn shade::Texture2D>,
	pub sprites_texture: Box<dyn shade::Texture2D>,
	pub sprites_atlas: shade::atlas::Atlas<chipty::SpriteId>,

	pub shader: Box<dyn shade::ShaderProgram>,
	pub shader_shadowmap: Box<dyn shade::ShaderProgram>,
	pub shader2d_pixelart: Box<dyn shade::ShaderProgram>,
	pub backbuffer_viewport: Bounds2i,

	pub colorshader: Box<dyn shade::ShaderProgram>,
	pub uishader: Box<dyn shade::ShaderProgram>,
	pub menubg: Box<dyn shade::Texture2D>,
	pub menubg_scale: f32,

	pub font: shade::d2::FontResource<shade::atlas::Font>,

	pub backcolor: Option<Box<dyn shade::Texture2D>>,
	pub backdepth: Option<Box<dyn shade::Texture2D>>,
	pub viewport: Bounds2i,
	pub pp: PostProcessEffect,
	pub post_process: crate::config::PostProcess,
	pub renderscale: f32,
}

#[track_caller]
fn load_png(
	g: &mut shade::Graphics,
	fs: &crate::FileSystem,
	path: &str,
	props: &shade::TextureProps,
) -> Result<Box<dyn shade::Texture2D>, shade::image::LoadImageError> {
	let data = fs.read(path).expect("Failed to read PNG file");
	let image = shade::image::DecodedImage::load_memory_png(data.as_slice())?;
	Ok(g.image(&props.bind(&image)))
}

impl Resources {
	pub fn backcolor(&self) -> &dyn shade::Texture2D {
		&**self.backcolor.as_ref().expect("back color texture has not been created")
	}

	pub fn backdepth(&self) -> &dyn shade::Texture2D {
		&**self.backdepth.as_ref().expect("back depth texture has not been created")
	}

	pub fn font_size(&self) -> f32 {
		self.viewport.width().min(self.viewport.height()) as f32 * crate::menu::FONT_SIZE
	}

	pub fn load(fs: &crate::FileSystem, config: &crate::config::Config, g: &mut shade::Graphics) -> Resources {
		let mut shaders = HashMap::<String, Box<dyn shade::ShaderProgram>>::new();
		for (name, shader) in &config.shaders {
			let (base, shader_name) = split_shader_path(&shader.shader);
			let mut interface = ShaderFsInterface { fs, base };
			let shader = g.shader_compile(&mut interface, shader_name, &[]);
			shaders.insert(name.to_string(), shader);
		}
		let mut textures = HashMap::<String, Box<dyn shade::Texture2D>>::new();
		for (name, texture) in &config.textures {
			let texture = load_png(g, fs, &texture.path, &texture.props).expect("Failed to load texture");
			textures.insert(name.to_string(), texture);
		}
		let font = {
			let font_config = config.fonts.get("Default").expect("Default font is not configured");
			let font = fs.read_to_string(&font_config.meta).expect("Failed to read font meta file");
			let font: shade::msdfgen::FontDto = serde_json::from_str(&font).expect("Failed to parse font meta file");
			let font: shade::atlas::Font = font.into();
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
			let shader = compile_static_shader(g, "mtsdf.glsl", shade::shaders::MTSDF);
			shade::d2::FontResource { font, shader, texture }
		};

		let sprite_atlas = fs.read_to_string("spritesheet.json").expect("Failed to read sprite atlas metadata");
		let sprites_atlas = serde_json::from_str(&sprite_atlas).expect("Failed to parse sprite atlas metadata");

		Resources {
			effects: textures.remove("Effects").expect("Effects texture is not configured"),
			sprites_texture: textures.remove("SpriteSheet").expect("SpriteSheet texture is not configured"),
			sprites_atlas,

			shader: shaders.remove("PixelArt").expect("PixelArt shader is not configured"),
			shader_shadowmap: shaders.remove("PixelArtShadowMap").expect("PixelArtShadowMap shader is not configured"),
			shader2d_pixelart: compile_static_shader(g, "pixelart.glsl", shade::shaders::PIXELART),
			backbuffer_viewport: Bounds2i::ZERO,

			colorshader: shaders.remove("Color").expect("Color shader is not configured"),
			uishader: shaders.remove("UI").expect("UI shader is not configured"),
			menubg: textures.remove("MenuBG").expect("MenuBG texture is not configured"),
			menubg_scale: 2.0 * config.render_scale,

			font,

			backcolor: None,
			backdepth: None,
			viewport: Bounds2i::ZERO,
			pp: PostProcessEffect {
				quad: shade::d2::PostProcessQuad::create(g),
				shader_copy: compile_static_shader(g, "post_process.copy.glsl", shade::shaders::POST_PROCESS_COPY),
				shader_crt: compile_static_shader(g, "post_process.crt.glsl", shade::shaders::POST_PROCESS_CRT),
			},
			post_process: config.post_process,
			renderscale: config.render_scale,
		}
	}

	pub fn update_back(&mut self, g: &mut shade::Graphics) {
		let width = (self.backbuffer_viewport.width() as f32 * self.renderscale) as i32;
		let height = (self.backbuffer_viewport.height() as f32 * self.renderscale) as i32;
		self.viewport = Bounds2!(0, 0, width, height);
		let color_info = shade::Texture2DInfo {
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
		};
		g.texture2d_ensure(&mut self.backcolor, &color_info);
		let depth_info = shade::Texture2DInfo {
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
		};
		g.texture2d_ensure(&mut self.backdepth, &depth_info);
	}

	pub fn present(&self, g: &mut shade::Graphics, time: f64) {
		g.begin(&shade::BeginArgs::BackBuffer {
			viewport: self.backbuffer_viewport,
		});
		match self.post_process {
			crate::config::PostProcess::None => {
				self.pp.quad.draw(g, &*self.pp.shader_copy, shade::BlendMode::Solid, &[
					&shade::shaders::PostProcessCopyUniforms {
						texture: self.backcolor(),
					}
				]);
			}
			crate::config::PostProcess::Crt => {
				self.pp.quad.draw(g, &*self.pp.shader_crt, shade::BlendMode::Solid, &[
					&shade::shaders::PostProcessCrtUniforms {
						texture: self.backcolor(),
						scanline_count: self.viewport.height() as f32 * 0.25,
						rgb_shift: 0.0,
						time: time as f32,
						..Default::default()
					}
				]);
			}
		}
		g.end();
	}
}
