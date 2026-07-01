#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::{fs, path, time};
use std::ffi::CString;
use std::num::NonZeroU32;

use glutin::prelude::*;

use chipgame::editor;
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
	cursor: winit::window::CursorIcon,
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
				.with_title("ChipEdit")
				.with_inner_size(size)
				.with_drag_and_drop(false)
		};
		#[cfg(not(windows))]
		let window_attributes = winit::window::Window::default_attributes()
			.with_title("ChipEdit")
			.with_inner_size(size);

		fn config_picker(configs: impl Iterator<Item = glutin::config::Config>) -> Option<glutin::config::Config> {
			configs
				.filter(|c| c.srgb_capable())
				.max_by_key(|c| c.num_samples())
		}

		let (window, gl_config) = glutin_winit::DisplayBuilder::new()
			.with_window_attributes(Some(window_attributes))
			.build(event_loop, template, |configs| config_picker(configs).expect("No suitable GL config found"))
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

		// Load GL functions for shaders/textures
		shade::gl::capi::load_with(|s| {
			let c = CString::new(s).unwrap();
			gl_display.get_proc_address(&c)
		});

		// Now that GL is ready, create graphics and resources
		let mut g = shade::gl::GlGraphics::new(shade::gl::GlConfig { srgb: true, ..Default::default() });
		let mut resx = chipgame::fx::Resources::load(fs, config, &mut g);
		resx.backbuffer_viewport.maxs = [size.width as i32, size.height as i32].into();
		resx.viewport = resx.backbuffer_viewport;

		let cursor = winit::window::CursorIcon::Default;
		Box::new(AppStuff { size, window, cursor, surface, context, g, resx })
	}

	fn set_fullscreen(&self, fullscreen: bool) {
		// Borderless fullscreen on the current monitor
		let target = if fullscreen {
			let monitor = self.window.current_monitor();
			Some(winit::window::Fullscreen::Borderless(monitor))
		}
		else {
			None
		};
		self.window.set_fullscreen(target);
	}
}

fn load_level(
	editor: &mut editor::EditorState,
	file_path: &Option<path::PathBuf>,
	app: Option<&AppStuff>,
	saved_level: &mut Option<String>,
) {
	if let Some(fp) = file_path {
		match fs::read_to_string(fp) {
			Ok(contents) => {
				editor.load_level(&contents);
				*saved_level = Some(editor.save_level());
				if let Some(app) = app {
					app.window.set_title(&format!("ChipEdit - {}", fp.display()));
				}
			}
			Err(err) => {
				eprintln!("Failed to load level {}: {err}", fp.display());
			}
		}
	}
}

fn init_audio(fs: &FileSystem, config: &chipgame::config::Config) -> Option<audio::AudioPlayer> {
	let mut ap = audio::AudioPlayer::create();
	ap.load(fs, config);
	Some(ap)
}

fn main() {
	#[cfg(all(windows, debug_assertions))]
	eprintln!("Warning: Windows debug build may slow down from print spam. Run with 2>NUL to suppress.");

	let time_base = time::Instant::now();

	// CLI: optional level path
	let matches = clap::command!()
		.arg(clap::arg!([level] "Level file to open").value_parser(clap::value_parser!(path::PathBuf)))
		.get_matches();

	let mut file_path = matches.get_one::<path::PathBuf>("level").cloned();

	let mut config = {
		let config = fs::read_to_string("chipdx.ini").unwrap_or_default();
		chipgame::config::Config::parse(config.as_str())
	};
	// Override certain config options that don't make sense for the editor
	config.post_process = chipgame::config::PostProcess::None;

	// VFS
	let key = paks::Key::default();
	let fs = if let Ok(paks) = paks::FileReader::open("data.paks", &key) {
		FileSystem::Paks(paks, key)
	}
	else {
		FileSystem::StdFs(path::PathBuf::from("data"))
	};

	let mut gamepads = gamepad::GamepadManager::new();

	let mut ap = init_audio(&fs, &config);

	// App state
	let mut app: Option<Box<AppStuff>> = None;
	let mut editor = editor::EditorState::default();
	editor.set_screen_size(800, 600);
	editor.load_level(include_str!("template.json"));
	let mut saved_level: Option<String> = None;

	let mut shift_held = false;
	let mut key_left = false;
	let mut key_right = false;
	let mut key_up = false;
	let mut key_down = false;
	let mut gamepad_start = false;
	let mut music_enabled = true;
	let mut last_frame_start = time::Instant::now();
	let mut tick_budget = time::Duration::ZERO;
	let mut fullscreen_toggled = false;

	let event_loop = winit::event_loop::EventLoop::new().expect("Failed to create event loop");

	#[allow(deprecated)]
	let _ = event_loop.run(move |event, event_loop| {
		use winit::event::{ElementState, Event, MouseButton, WindowEvent};
		use winit::keyboard::{KeyCode, PhysicalKey};
		use winit::window::CursorIcon;

		match event {
			Event::Resumed => {
				if app.is_none() {
					let built = AppStuff::new(event_loop, &fs, &config);

					// Window title and initial level
					built.window.set_title("ChipEdit - (unsaved)");
					app = Some(built);

					load_level(&mut editor, &file_path, app.as_deref(), &mut saved_level);
					last_frame_start = time::Instant::now() - FRAME_TIME;
					tick_budget = FRAME_TIME / 2; // Start biased to avoid jitter-induced double ticks
				}
			}
			Event::WindowEvent { event, .. } => match event {
				WindowEvent::ModifiersChanged(new_mods) => {
					shift_held = new_mods.state().shift_key();
				}
				WindowEvent::Resized(new_size) => {
					if let Some(app) = app.as_deref_mut() {
						let width = NonZeroU32::new(new_size.width.max(1)).unwrap();
						let height = NonZeroU32::new(new_size.height.max(1)).unwrap();
						app.size = new_size;
						app.surface.resize(&app.context, width, height);
						app.resx.backbuffer_viewport.maxs = [app.size.width as i32, app.size.height as i32].into();
					}
					editor.set_screen_size(new_size.width as i32, new_size.height as i32);
				}
				WindowEvent::CloseRequested => {
					let current_level = editor.save_level();
					let has_unsaved = match &saved_level {
						Some(saved) => *saved != current_level,
						None => true,
					};

					if has_unsaved {
						let result = rustydialogs::MessageBox {
							title: "Unsaved changes",
							message: "There are unsaved changes. Quit without saving?",
							icon: rustydialogs::MessageIcon::Warning,
							buttons: rustydialogs::MessageButtons::YesNo,
							owner: window_owner(app.as_deref()),
						}.show();
						if result == Some(rustydialogs::MessageResult::Yes) {
							event_loop.exit();
						}
					}
					else {
						event_loop.exit();
					}
				}
				WindowEvent::KeyboardInput { event, .. } => {
					let pressed = matches!(event.state, ElementState::Pressed);

					match event.physical_key {
						PhysicalKey::Code(KeyCode::ArrowLeft) => key_left = pressed,
						PhysicalKey::Code(KeyCode::ArrowRight) => key_right = pressed,
						PhysicalKey::Code(KeyCode::ArrowUp) => key_up = pressed,
						PhysicalKey::Code(KeyCode::ArrowDown) => key_down = pressed,
						PhysicalKey::Code(KeyCode::Enter) if pressed => editor.toggle_play(),
						PhysicalKey::Code(KeyCode::Delete) => editor.delete(pressed),
						PhysicalKey::Code(KeyCode::KeyU) if pressed => editor.undo(),
						PhysicalKey::Code(KeyCode::KeyY) if pressed => editor.redo(),
						PhysicalKey::Code(KeyCode::F2) if pressed => {
							// Open file dialog to load a level
							let open_path = rustydialogs::FileDialog {
								title: "Open Level",
								path: file_path.as_deref(),
								filter: Some(&[
									rustydialogs::FileFilter {
										desc: "Level Files",
										patterns: &["*.json"],
									},
								]),
								owner: window_owner(app.as_deref()),
							}.pick_file();
							if let Some(path) = open_path {
								file_path = Some(path::PathBuf::from(path));
								load_level(&mut editor, &file_path, app.as_deref(), &mut saved_level);
							}
						}
						PhysicalKey::Code(KeyCode::F5) if pressed => {
							// Save file dialog to save the level
							let save_path = rustydialogs::FileDialog {
								title: "Save Level",
								path: file_path.as_deref(),
								filter: Some(&[
									rustydialogs::FileFilter {
										desc: "Level Files",
										patterns: &["*.json"],
									},
								]),
								owner: window_owner(app.as_deref()),
							}.save_file();
							if let Some(path) = save_path {
								let path = path::PathBuf::from(path);
								let contents = editor.save_level();
								saved_level = Some(contents.clone());
								match fs::write(&path, contents) {
									Ok(_) => {
										file_path = Some(path);
										if let Some(app) = &app {
											app.window.set_title(&format!("ChipEdit - {}", file_path.as_ref().unwrap().display()));
										}
									}
									Err(err) => {
										eprintln!("Failed to save level: {err}");
									}
								}
							}
						}
						PhysicalKey::Code(KeyCode::F6) if pressed => {
							if let editor::EditorState::Edit(editor) = &mut editor {
								let current = editor.save_level_dto().name;
								let result = rustydialogs::TextInput {
									title: "Level Title",
									message: "Enter the level title:",
									value: &current,
									mode: rustydialogs::TextInputMode::SingleLine,
									owner: window_owner(app.as_deref()),
								}.show();
								if let Some(value) = result {
									editor.set_level_name(value);
								}
							}
						}
						PhysicalKey::Code(KeyCode::F7) if pressed => {
							if let editor::EditorState::Edit(editor) = &mut editor {
								let current = editor.save_level_dto().required_chips;
								let result = rustydialogs::TextInput {
									title: "Required Chips",
									message: "Enter how many chips are required:",
									value: &current.to_string(),
									mode: rustydialogs::TextInputMode::SingleLine,
									owner: window_owner(app.as_deref()),
								}.show();
								if let Some(value) = result {
									match value.trim_ascii().parse::<i32>() {
										Ok(number) => editor.set_required_chips(number),
										Err(_) => eprintln!("Invalid numeric input for required chips"),
									}
								}
							}
						}
						PhysicalKey::Code(KeyCode::F8) if pressed => {
							if let editor::EditorState::Edit(editor) = &mut editor {
								let current = editor.save_level_dto().time_limit;
								let result = rustydialogs::TextInput {
									title: "Time Limit",
									message: "Enter the time limit in ticks (0 for no limit):",
									value: &current.to_string(),
									mode: rustydialogs::TextInputMode::SingleLine,
									owner: window_owner(app.as_deref()),
								}.show();
								if let Some(value) = result {
									match value.trim_ascii().parse::<i32>() {
										Ok(number) => editor.set_time_limit(number),
										Err(_) => eprintln!("Invalid numeric input for time limit"),
									}
								}
							}
						}
						PhysicalKey::Code(KeyCode::F9) if pressed => {
							if let editor::EditorState::Edit(editor) = &mut editor {
								let current = editor.save_level_dto().hint;
								let result = rustydialogs::TextInput {
									title: "Level Hint",
									message: "Enter the level hint:",
									value: current.as_deref().unwrap_or(""),
									mode: rustydialogs::TextInputMode::MultiLine,
									owner: window_owner(app.as_deref()),
								}.show();
								if let Some(value) = result {
									let hint = value.trim_ascii();
									let hint = if hint.is_empty() { None } else { Some(value) };
									editor.set_hint(hint);
								}
							}
						}
						PhysicalKey::Code(KeyCode::F10) if pressed => {
							if let editor::EditorState::Edit(editor) = &mut editor {
								let current = editor.save_level_dto().author;
								let result = rustydialogs::TextInput {
									title: "Author Name",
									message: "Enter the author name:",
									value: current.as_deref().unwrap_or(""),
									mode: rustydialogs::TextInputMode::SingleLine,
									owner: window_owner(app.as_deref()),
								}.show();
								if let Some(value) = result {
									let author = value.trim_ascii();
									let author = if author.is_empty() { None } else { Some(value) };
									editor.set_author(author);
								}
							}
						}
						PhysicalKey::Code(KeyCode::KeyT) => editor.tool_terrain(pressed),
						PhysicalKey::Code(KeyCode::KeyE) => editor.tool_entity(pressed),
						PhysicalKey::Code(KeyCode::KeyC) => editor.tool_connection(pressed),
						PhysicalKey::Code(KeyCode::KeyI) => editor.tool_icepath(pressed),
						PhysicalKey::Code(KeyCode::KeyR) => editor.tool_forcepath(pressed),
						PhysicalKey::Code(KeyCode::KeyO) => editor.tool_entorder(pressed),
						PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => editor.key_shift(pressed),
						PhysicalKey::Code(KeyCode::Numpad8) if pressed => {
							if shift_held { editor.expand_top(); }
							else { editor.crop_top(); }
							let level = editor.save_level();
							editor.reload_level(&level);
						}
						PhysicalKey::Code(KeyCode::Numpad2) if pressed => {
							if shift_held { editor.expand_bottom(); }
							else { editor.crop_bottom(); }
							let level = editor.save_level();
							editor.reload_level(&level);
						}
						PhysicalKey::Code(KeyCode::Numpad4) if pressed => {
							if shift_held { editor.expand_left(); }
							else { editor.crop_left(); }
							let level = editor.save_level();
							editor.reload_level(&level);
						}
						PhysicalKey::Code(KeyCode::Numpad6) if pressed => {
							if shift_held { editor.expand_right(); }
							else { editor.crop_right(); }
							let level = editor.save_level();
							editor.reload_level(&level);
						}
						PhysicalKey::Code(KeyCode::NumpadAdd) if pressed => {
							editor.zoom_in();
						}
						PhysicalKey::Code(KeyCode::NumpadSubtract) if pressed => {
							editor.zoom_out();
						}
						PhysicalKey::Code(KeyCode::KeyM) if pressed => {
							music_enabled = !music_enabled;
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
				WindowEvent::MouseInput { state, button, .. } => {
					let pressed = matches!(state, ElementState::Pressed);

					match button {
						MouseButton::Left => editor.left_click(pressed),
						MouseButton::Right => editor.right_click(pressed),
						_ => {}
					}
				}
				WindowEvent::CursorMoved { position, .. } => {
					editor.mouse_move(position.x as i32, position.y as i32);
				}
				WindowEvent::RedrawRequested => {
					if let Some(app) = app.as_deref_mut() {
						let frame_start = time::Instant::now();
						let frame_dt = frame_start - last_frame_start;
						last_frame_start = frame_start;

						// Gamepad support
						let pad_input = gamepads.poll();
						editor.key_left(key_left || pad_input.left);
						editor.key_right(key_right || pad_input.right);
						editor.key_up(key_up || pad_input.up);
						editor.key_down(key_down || pad_input.down);
						if !gamepad_start && pad_input.start {
							editor.toggle_play();
						}
						gamepad_start = pad_input.start;

						let tool_cursor = match editor.get_tool() {
							Some(editor::ToolState::Terrain(_)) => CursorIcon::Crosshair,
							Some(editor::ToolState::Entity(_)) => CursorIcon::Pointer,
							Some(editor::ToolState::Connection(_)) => CursorIcon::Alias,
							Some(editor::ToolState::IcePath(_)) => CursorIcon::Crosshair,
							Some(editor::ToolState::ForcePath(_)) => CursorIcon::Crosshair,
							Some(editor::ToolState::EntOrder(_)) => CursorIcon::NsResize,
							None => CursorIcon::Default,
						};

						if app.cursor != tool_cursor {
							app.cursor = tool_cursor;
							app.window.set_cursor(tool_cursor);
						}

						tick_budget += frame_dt;
						while tick_budget >= FRAME_TIME {
							editor.think();
							tick_budget -= FRAME_TIME;
						}

						let fx_events = editor.take_fx_events();
						for evt in fx_events {
							match evt {
								chipgame::fx::FxEvent::PlaySound(sound) => if let Some(ap) = &mut ap { ap.play_sound(sound) },
								chipgame::fx::FxEvent::LevelComplete => level_complete(&mut editor, Some(app)),
								chipgame::fx::FxEvent::GameOver => editor.toggle_play(),
								_ => {}
							}
						}

						let music = editor.get_music(music_enabled);
						if let Some(ap) = &mut ap { ap.play_music(music); }

						let time = time_base.elapsed().as_secs_f64();
						app.resx.update_back(&mut app.g);
						editor.draw(&mut app.g, &app.resx, time);
						app.resx.present(&mut app.g, time);

						_ = app.g.get_draw_metrics(true);

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

fn level_complete(editor: &mut editor::EditorState, app: Option<&AppStuff>) {
	let string;
	let description = if let Some(run_stats) = editor.play_stats() {
		string = format!(
			"RealTime: {:.2} sec\nTime: {} ticks\nTime: {}\nSteps: {}\nBonks: {}\n\nEmbed this run into the level file for future reference?",
			run_stats.realtime, run_stats.ticks,
			chipcore::FmtTime::new(&run_stats.ticks),
			run_stats.steps, run_stats.bonks,
		);
		&string
	}
	else {
		"Embed this run into the level file for future reference?"
	};
	let result = rustydialogs::MessageBox {
		title: "Save replay?",
		message: description,
		icon: rustydialogs::MessageIcon::Info,
		buttons: rustydialogs::MessageButtons::YesNo,
		owner: window_owner(app.as_deref()),
	}.show();
	if result == Some(rustydialogs::MessageResult::Yes) {
		editor.save_replay();
	}
	editor.toggle_play();
}

fn window_owner(app: Option<&AppStuff>) -> Option<&dyn raw_window_handle::HasWindowHandle> {
	app.map(|a| &a.window as &dyn raw_window_handle::HasWindowHandle)
}

// Dummy file I/O functions for linking
#[no_mangle]
extern "C" fn chipgame_write_file(_path_ptr: *const u8, _path_len: usize, _content_ptr: *const u8, _content_len: usize) -> i32 { -1 }
#[no_mangle]
extern "C" fn chipgame_read_file(_path_ptr: *const u8, _path_len: usize, _content_ptr: *mut String) -> i32 { -1 }
