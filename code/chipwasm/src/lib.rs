// #![cfg(target_family = "wasm")]

use std::{mem, ptr, str};

mod api;

const CHIPDX_INI: &str = include_str!("../../../chipdx.webgl.ini");
paks::static_bundle!(DATA_PAK = concat!(env!("OUT_DIR"), "/data.paks"));
paks::static_bundle!(CCLP1_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp1.paks"));
paks::static_bundle!(CCLP2_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp2.paks"));
paks::static_bundle!(CCLP3_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp3.paks"));
paks::static_bundle!(CCLP4_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp4.paks"));
paks::static_bundle!(CCLP5_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp5.paks"));

fn play_sound(sound: chipty::SoundFx) {
	unsafe {
		api::playSound(sound as i32);
	}
}
fn play_music(music: Option<chipty::MusicId>) {
	let id = music.map(|m| m as i32).unwrap_or(-1);
	unsafe {
		api::playMusic(id);
	}
}
fn set_title(state: &chipgame::play::PlayState) {
	let title = if let Some(fx) = &state.fx {
		format!("{} - Level {} - {}", state.lvsets.current().title, fx.level_number, fx.game.field.name)
	}
	else if let Some(level_set) = state.lvsets.collection.get(state.lvsets.selected as usize) {
		level_set.title.clone()
	}
	else {
		"Choose LevelSet".to_string()
	};
	unsafe {
		api::setTitle(title.as_ptr(), title.len());
	}
}
fn quit_game(instance: &mut Instance) {
	// Relaunch the game to return to the levelset select screen, since we can't actually quit the page
	instance.play.lvsets.selected = -1;
	instance.play.launch(instance.graphics.as_graphics());
	set_title(&instance.play);
}

pub struct Instance {
	graphics: shade::webgl::WebGLGraphics,
	resx: chipgame::fx::Resources,
	play: chipgame::play::PlayState,
}

fn create_instance() -> Box<Instance> {
	let mut instance = Box::new(Instance {
		graphics: shade::webgl::WebGLGraphics::new(shade::webgl::WebGLConfig {
			srgb: false,
		}),
		resx: chipgame::fx::Resources::default(),
		play: chipgame::play::PlayState::default(),
	});

	let config = chipgame::config::Config::parse(CHIPDX_INI);
	let key = paks::Key::default();
	let paks = paks::BundleReader::open(&DATA_PAK, key).expect("Failed to open data.paks");
	let fs = chipgame::FileSystem::Bundle(paks);
	instance.resx.load(&fs, &config, instance.graphics.as_graphics());
	return instance;
}

fn load_levelset(data: &'static [paks::Block], name: String, play: &mut chipgame::play::PlayState) {
	let key = paks::Key::default();
	let paks = paks::BundleReader::open(data, key).expect("Failed to open levelset paks");
	let fs = chipgame::FileSystem::Bundle(paks);
	chipgame::play::load_levelset(&fs, name, &mut play.lvsets.collection);
}

#[no_mangle]
pub extern "C" fn createInstance() -> *mut Instance {
	shade::webgl::setup_panic_hook();

	let mut instance = create_instance();

	load_levelset(&CCLP1_PAK, "cclp1".to_string(), &mut instance.play);
	load_levelset(&CCLP2_PAK, "cclp2".to_string(), &mut instance.play);
	load_levelset(&CCLP3_PAK, "cclp3".to_string(), &mut instance.play);
	load_levelset(&CCLP4_PAK, "cclp4".to_string(), &mut instance.play);
	load_levelset(&CCLP5_PAK, "cclp5".to_string(), &mut instance.play);

	instance.play.launch(instance.graphics.as_graphics());

	Box::into_raw(instance)
}

fn load_custom_level(mut level: chipty::LevelDto) -> Box<Instance> {
	level.normalize();
	let level_set = chipgame::play::LevelSet {
		name: "Custom Level".to_string(),
		title: "Custom Level".to_string(),
		about: None,
		splash: None,
		levels: vec![level],
	};

	let mut instance = create_instance();
	instance.play.save_data.ephemeral = true;
	instance.play.load_single_level(level_set);
	instance.play.play_level(1);
	return instance;
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn createCustomLevel(level_ptr: *const u8, level_len: usize, compressed: bool) -> *mut Instance {
	shade::webgl::setup_panic_hook();

	if level_ptr.is_null() {
		api::result_error("Missing custom level payload");
		return ptr::null_mut();
	}

	let level = unsafe { std::slice::from_raw_parts(level_ptr, level_len) };

	let level_data;
	let level_json = if compressed {
		let Ok(level_input) = str::from_utf8(level) else {
			api::result_error("Compressed custom level is not valid UTF-8");
			return ptr::null_mut();
		};
		let Some(decoded) = chipty::try_decode_level(level_input) else {
			api::result_error("Compressed custom level could not be decoded");
			return ptr::null_mut();
		};
		level_data = decoded;
		level_data.as_slice()
	}
	else {
		level
	};

	let Ok(level_json) = str::from_utf8(level_json) else {
		api::result_error("Custom level is not valid UTF-8");
		return ptr::null_mut();
	};
	let Ok(level) = serde_json::from_str::<chipty::LevelDto>(level_json) else {
		api::result_error("Custom level JSON is invalid");
		return ptr::null_mut();
	};

	let instance = load_custom_level(level);
	Box::into_raw(instance)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn audioInit() {
	let config = chipgame::config::Config::parse(CHIPDX_INI);
	let key = paks::Key::default();
	let paks = paks::BundleReader::open(&DATA_PAK, key).expect("Failed to open data.paks");
	let fs = chipgame::FileSystem::Bundle(paks);
	register_audio_assets(&fs, &config);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn allocBytes(len: usize) -> *mut u8 {
	let mut buf = Vec::<u8>::with_capacity(len);
	let ptr = buf.as_mut_ptr();
	mem::forget(buf);
	ptr
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn freeBytes(ptr: *mut u8, len: usize) {
	if ptr.is_null() {
		return;
	}
	unsafe {
		let _ = Vec::from_raw_parts(ptr, 0, len);
	}
}

fn register_audio_assets(fs: &chipgame::FileSystem, config: &chipgame::config::Config) {
	for (&fx, path) in &config.sound_fx {
		if let Ok(data) = fs.read(path) {
			unsafe {
				api::registerSound(fx as i32, data.as_ptr(), data.len());
			}
		}
	}
	for (&music, path) in &config.music {
		if let Ok(data) = fs.read(path) {
			unsafe {
				api::registerMusic(music as i32, data.as_ptr(), data.len());
			}
		}
	}
}


#[no_mangle]
pub extern "C" fn destroyInstance(instance: *mut Instance) {
	_ = unsafe { Box::from_raw(instance) };
}

#[no_mangle]
pub extern "C" fn thinkInstance(instance: *mut Instance, buttons: u8) {
	let instance = unsafe { &mut *instance };
	let input = chipcore::Input::decode(buttons);
	instance.play.think(&input);

	for evt in &mem::replace(&mut instance.play.events, Vec::new()) {
		match evt {
			&chipgame::play::PlayEvent::PlaySound { sound } => play_sound(sound),
			&chipgame::play::PlayEvent::PlayMusic { music } => play_music(music),
			&chipgame::play::PlayEvent::SetTitle => set_title(&instance.play),
			&chipgame::play::PlayEvent::Restart => instance.play.launch(instance.graphics.as_graphics()),
			&chipgame::play::PlayEvent::Quit => quit_game(instance),
		}
	}
}

#[no_mangle]
pub extern "C" fn drawInstance(instance: *mut Instance, time: f64, width: i32, height: i32) {
	let instance = unsafe { &mut *instance };
	let g = instance.graphics.as_graphics();
	instance.resx.backbuffer_viewport.maxs = cvmath::Vec2i(width, height);
	instance.resx.update_back(g);
	instance.play.draw(g, &instance.resx, time);
	instance.resx.present(g);
}

#[no_mangle]
extern "C" fn chipgame_write_file(path_ptr: *const u8, path_len: usize, content_ptr: *const u8, content_len: usize) -> i32 {
	unsafe {
		api::writeFile(path_ptr, path_len, content_ptr, content_len)
	}
}

#[no_mangle]
extern "C" fn chipgame_read_file(path_ptr: *const u8, path_len: usize, content_ptr: *mut String) -> i32 {
	unsafe {
		let mut size: usize = 0;
		if api::readFile(path_ptr, path_len, ptr::null_mut(), &mut size as *mut usize) != 0 {
			return -1;
		}
		let mut content = vec![0u8; size];
		let mut read = size;
		if api::readFile(path_ptr, path_len, content.as_mut_ptr(), &mut read as *mut usize) != 0 {
			return -1;
		}
		if read > content.len() {
			return -1;
		}
		content.truncate(read);
		let Ok(content) = String::from_utf8(content) else {
			return -1;
		};
		*content_ptr = content;
		return 0;
	}
}
