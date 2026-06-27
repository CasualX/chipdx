#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::{fs, mem, path, time};
use std::ffi::CString;
use std::num::NonZeroU32;

use glutin::prelude::*;

use chipgame::FileSystem;

mod audio;
mod gamepad;

const NANOS_PER_SECOND: f64 = 1_000_000_000.0;
const FRAME_TIME: time::Duration = time::Duration::from_nanos((NANOS_PER_SECOND / chipcore::FPS as f64) as u64);

struct AppStuff {
	size: winit::dpi::PhysicalSize<u32>,
	resx: chipgame::fx::Resources,
	g: shade::gl::GlGraphics,
	context: glutin::context::PossiblyCurrentContext,
	surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
	window: winit::window::Window,
}

impl AppStuff {
	fn new(
		event_loop: &winit::event_loop::ActiveEventLoop,
		fs: &FileSystem,
		config: &chipgame::config::Config,
	) -> Box<AppStuff> {
		use glutin::config::ConfigTemplateBuilder;
		use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
		use glutin::display::GetGlDisplay;
		use glutin::surface::{SurfaceAttributesBuilder, SwapInterval, WindowSurface};
		use raw_window_handle::HasWindowHandle;

		let mut template = ConfigTemplateBuilder::new().with_alpha_size(8);
		if config.multisampling > 0 {
			template = template.with_multisampling(config.multisampling);
		}

		let size = winit::dpi::PhysicalSize::new(800, 600);

		#[cfg(windows)]
		let window_attributes = {
			use winit::platform::windows::WindowAttributesExtWindows;
			winit::window::Window::default_attributes()
				.with_title("Chip's Challenge DX")
				.with_inner_size(size)
				.with_drag_and_drop(false)
		};

		#[cfg(not(windows))]
		let window_attributes = winit::window::Window::default_attributes()
			.with_title("Chip's Challenge DX")
			.with_inner_size(size);

		let (window, gl_config) = glutin_winit::DisplayBuilder::new()
			.with_window_attributes(Some(window_attributes))
			.build(event_loop, template, |configs| configs.max_by_key(|c| c.num_samples()).unwrap())
			.expect("Failed to build window and GL config");

		let window = window.expect("DisplayBuilder did not build a Window");
		let raw_window_handle = window
			.window_handle()
			.expect("Failed to get window handle")
			.as_raw();

		let context_attributes = ContextAttributesBuilder::new()
			.with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
			.build(Some(raw_window_handle));

		let gl_display = gl_config.display();

		let not_current = unsafe {
			gl_display.create_context(&gl_config, &context_attributes)
		}.expect("Failed to create GL context");

		let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
			raw_window_handle,
			NonZeroU32::new(size.width.max(1)).unwrap(),
			NonZeroU32::new(size.height.max(1)).unwrap(),
		);

		let surface = unsafe {
			gl_display.create_window_surface(&gl_config, &attrs)
		}.expect("Failed to create GL surface");

		let context = not_current
			.make_current(&surface)
			.expect("Failed to make GL context current");

		if config.vsync {
			if let Err(err) = surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap())) {
				eprintln!("Failed to enable vsync: {err}");
			}
		}

		// Load GL function pointers
		shade::gl::capi::load_with(|s| {
			let c = CString::new(s).unwrap();
			gl_display.get_proc_address(&c)
		});

		// Now that GL is ready, create graphics and resources
		let mut g = shade::gl::GlGraphics::new(shade::gl::GlConfig { srgb: true, ..Default::default() });
		let mut resx = chipgame::fx::Resources::load(fs, config, &mut g);
		resx.backbuffer_viewport.maxs = [size.width as i32, size.height as i32].into();
		resx.viewport = resx.backbuffer_viewport;

		Box::new(AppStuff { size, window, surface, context, g, resx })
	}

	fn set_title(&self, state: &chipgame::play::PlayState) {
		if let Some(fx) = &state.fx {
			self.window.set_title(&format!("{} - Level {} - {}", state.lvsets.current().title, fx.level_number, fx.game.field.name));
		}
		else if let Some(level_set) = state.lvsets.collection.get(state.lvsets.selected as usize) {
			self.window.set_title(&level_set.title);
		}
		else {
			self.window.set_title("Choose LevelSet");
		}
	}

	fn set_fullscreen(&self, fullscreen: bool) {
		// Borderless fullscreen on the current monitor; hide cursor when fullscreen
		let target = if fullscreen {
			let monitor = self.window.current_monitor();
			Some(winit::window::Fullscreen::Borderless(monitor))
		}
		else {
			None
		};
		self.window.set_fullscreen(target);
		self.window.set_cursor_visible(!fullscreen);
	}
}

fn main() {
	let time_base = time::Instant::now();

	let config = {
		let config = fs::read_to_string("chipdx.ini").unwrap_or_default();
		chipgame::config::Config::parse(config.as_str())
	};

	// VFS
	let key = paks::Key::default();
	let fs = if let Ok(paks) = paks::FileReader::open("data.paks", &key) {
		FileSystem::Paks(paks, key)
	}
	else {
		FileSystem::StdFs(path::PathBuf::from("data"))
	};

	let mut gamepads = gamepad::GamepadManager::new();

	let mut ap = audio::AudioPlayer::create();
	ap.load(&fs, &config);

	// App state
	let mut app: Option<Box<AppStuff>> = None;
	let mut state = Box::new(chipgame::play::PlayState::default());
	state.lvsets.load();

	let mut key_input = chipcore::Input::default();
	let mut last_frame_start = time::Instant::now();
	let mut tick_budget = time::Duration::ZERO;
	let mut fullscreen_toggled = false;

	let event_loop = winit::event_loop::EventLoop::new().expect("Failed to create event loop");

	#[allow(deprecated)]
	let _ = event_loop.run(move |event, event_loop| {
		use winit::event::{ElementState, Event, WindowEvent};
		use winit::keyboard::{KeyCode, PhysicalKey};

		match event {
			Event::Resumed => {
				if app.is_none() {
					let mut built = AppStuff::new(event_loop, &fs, &config);
					state.launch(&mut built.g);
					built.set_title(&state);
					app = Some(built);
					last_frame_start = time::Instant::now() - FRAME_TIME;
					tick_budget = FRAME_TIME / 2; // Start biased to avoid jitter-induced double ticks
				}
			}
			Event::WindowEvent { event, .. } => match event {
				WindowEvent::Resized(new_size) => {
					if let Some(app) = app.as_deref_mut() {
						let width = NonZeroU32::new(new_size.width.max(1)).unwrap();
						let height = NonZeroU32::new(new_size.height.max(1)).unwrap();
						app.size = new_size;
						app.surface.resize(&app.context, width, height);
						app.resx.backbuffer_viewport.maxs = [app.size.width as i32, app.size.height as i32].into();
					}
				}
				WindowEvent::CloseRequested => event_loop.exit(),
				WindowEvent::KeyboardInput { event, .. } => {
					let pressed = matches!(event.state, ElementState::Pressed);

					match event.physical_key {
						PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::KeyA) => key_input.left = pressed,
						PhysicalKey::Code(KeyCode::ArrowRight | KeyCode::KeyD) => key_input.right = pressed,
						PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::KeyW) => key_input.up = pressed,
						PhysicalKey::Code(KeyCode::ArrowDown | KeyCode::KeyS) => key_input.down = pressed,
						PhysicalKey::Code(KeyCode::Space) => key_input.a = pressed,
						PhysicalKey::Code(KeyCode::Backspace) => key_input.b = pressed,
						PhysicalKey::Code(KeyCode::Enter) => key_input.start = pressed,
						PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => key_input.select = pressed,
						PhysicalKey::Code(KeyCode::KeyM) if pressed => {
							state.toggle_music();
						}
						PhysicalKey::Code(KeyCode::KeyF) if pressed => {
							if let Some(app) = app.as_deref_mut() {
								if !fullscreen_toggled {
									fullscreen_toggled = true;
									let want_fullscreen = app.window.fullscreen().is_none();
									app.set_fullscreen(want_fullscreen);
								}
							}
						}
						PhysicalKey::Code(KeyCode::Escape) if pressed => {
							if let Some(app) = app.as_deref_mut() {
								if !fullscreen_toggled {
									fullscreen_toggled = true;
									app.set_fullscreen(false);
								}
							}
						}
						_ => {}
					}
				}
				WindowEvent::RedrawRequested => {
					if let Some(app) = app.as_deref_mut() {
						let frame_start = time::Instant::now();
						let frame_dt = frame_start - last_frame_start;
						last_frame_start = frame_start;

						// Gamepad support
						let pad_input = gamepads.poll();
						let input = key_input | pad_input;

						tick_budget += frame_dt;
						while tick_budget >= FRAME_TIME {
							state.think(&input);
							tick_budget -= FRAME_TIME;
						}

						for evt in &mem::replace(&mut state.events, Vec::new()) {
							match evt {
								&chipgame::play::PlayEvent::PlaySound { sound } => ap.play_sound(sound),
								&chipgame::play::PlayEvent::PlayMusic { music } => ap.play_music(music),
								&chipgame::play::PlayEvent::SetTitle => app.set_title(&state),
								&chipgame::play::PlayEvent::Restart => state.launch(&mut app.g),
								&chipgame::play::PlayEvent::LoadExternalLevelSet => (),
								&chipgame::play::PlayEvent::Quit => event_loop.exit(),
							}
						}

						let time = time_base.elapsed().as_secs_f64();
						app.resx.update_back(&mut app.g);
						state.draw(&mut app.g, &mut app.resx, time);
						app.resx.present(&mut app.g, time);

						state.metrics = app.g.get_draw_metrics(true);

						app.surface.swap_buffers(&app.context).unwrap();
					}
				}
				_ => {}
			},
			Event::AboutToWait => {
				fullscreen_toggled = false;
				if let Some(app) = app.as_deref() {
					app.window.request_redraw();
				}
			}
			_ => {}
		}
	});
}

// Dummy file I/O functions for linking
#[no_mangle]
extern "C" fn chipgame_write_file(_path_ptr: *const u8, _path_len: usize, _content_ptr: *const u8, _content_len: usize) -> i32 { -1 }
#[no_mangle]
extern "C" fn chipgame_read_file(_path_ptr: *const u8, _path_len: usize, _content_ptr: *mut String) -> i32 { -1 }
