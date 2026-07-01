use super::*;

pub struct PlayInstance {
	resx: chipgame::fx::Resources,
	graphics: shade::webgl::WebGLGraphics,
	play: chipgame::play::PlayState,
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

fn quit_game(instance: &mut PlayInstance) {
	// Relaunch the game to return to the levelset select screen, since we can't actually quit the page.
	instance.play.lvsets.selected = -1;
	instance.play.launch(instance.graphics.as_graphics());
	set_title(&instance.play);
}

fn request_levelset_file() {
	unsafe {
		api::requestLevelSetFile();
	}
}

fn create_instance() -> Box<PlayInstance> {
	let mut config = chipgame::config::Config::parse(CHIPDX_INI);
	config.render_scale = 0.5;
	let (graphics, resx) = create_graphics_resources(&config);
	let mut play = chipgame::play::PlayState::default();
	play.lvsets.external_loader_label = Some("Play Custom Levelset".to_string());
	Box::new(PlayInstance { graphics, resx, play })
}

fn load_levelset(data: &'static [paks::Block], name: String, play: &mut chipgame::play::PlayState) {
	let key = paks::Key::default();
	let paks = paks::BundleReader::open(data, key).expect("Failed to open levelset paks");
	let fs = chipgame::FileSystem::Bundle(paks);
	chipgame::play::load_levelset(&fs, name, &mut play.lvsets.collection);
}

#[no_mangle]
pub extern "C" fn createPlayInstance() -> *mut PlayInstance {
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

fn load_custom_level(mut level: chipty::LevelDto) -> Box<PlayInstance> {
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
pub extern "C" fn createCustomPlayLevel(level_ptr: *const u8, level_len: usize, compressed: bool) -> *mut PlayInstance {
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
pub extern "C" fn loadLocalPlayLevelSet(instance: *mut PlayInstance, data_ptr: *const u8, data_len: usize, title_ptr: *const u8, title_len: usize) -> i32 {
	if instance.is_null() {
		api::result_error("Missing game instance");
		return -1;
	}
	if data_ptr.is_null() {
		api::result_error("Missing levelset file data");
		return -1;
	}

	let data = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };
	let opts = chipdat::Options {
		encoding: chipdat::Encoding::Windows1252,
	};
	let dat = match chipdat::parse(data, &opts) {
		Ok(dat) => dat,
		Err(err) => {
			api::result_error(&format!("Could not parse levelset file: {err:?}"));
			return -1;
		}
	};

	let name_bytes = if title_ptr.is_null() { &[] } else { unsafe { std::slice::from_raw_parts(title_ptr, title_len) } };
	let title = str::from_utf8(name_bytes).unwrap_or("").to_string();

	let index = chipdat::convert(&dat, title.clone());
	if index.levels.is_empty() {
		api::result_error("Levelset file did not contain any playable levels");
		return -1;
	}
	let instance = unsafe { &mut *instance };
	chipgame::play::load_levelset_dto(None, index, title.clone(), &mut instance.play.lvsets.collection);
	let selected = instance.play.lvsets.collection.len() as i32 - 1;
	instance.play.fx = None;
	instance.play.warp = None;
	instance.play.lvsets.selected = selected;
	instance.play.save_data.load(instance.play.lvsets.current());
	instance.play.save_data.save(instance.play.lvsets.current());
	instance.play.menu.open_main(instance.play.save_data.current_level > 0, &instance.play.lvsets.current().title);
	instance.play.events.push(chipgame::play::PlayEvent::SetTitle);
	instance.play.events.push(chipgame::play::PlayEvent::PlayMusic { music: Some(chipty::MusicId::MenuMusic) });
	return 0;
}

#[no_mangle]
pub extern "C" fn destroyPlayInstance(instance: *mut PlayInstance) {
	if instance.is_null() {
		return;
	}
	_ = unsafe { Box::from_raw(instance) };
}

#[no_mangle]
pub extern "C" fn thinkPlayInstance(instance: *mut PlayInstance, buttons: u8) {
	let instance = unsafe { &mut *instance };
	let input = chipcore::Input::decode(buttons);
	instance.play.think(&input);

	for evt in &mem::replace(&mut instance.play.events, Vec::new()) {
		match evt {
			&chipgame::play::PlayEvent::PlaySound { sound } => play_sound(sound),
			&chipgame::play::PlayEvent::PlayMusic { music } => play_music(music),
			&chipgame::play::PlayEvent::SetTitle => set_title(&instance.play),
			&chipgame::play::PlayEvent::Restart => instance.play.launch(instance.graphics.as_graphics()),
			&chipgame::play::PlayEvent::LoadExternalLevelSet => request_levelset_file(),
			&chipgame::play::PlayEvent::Quit => quit_game(instance),
		}
	}
}

#[no_mangle]
pub extern "C" fn drawPlayInstance(instance: *mut PlayInstance, time: f64, width: i32, height: i32) {
	let instance = unsafe { &mut *instance };
	let g = instance.graphics.as_graphics();
	instance.resx.backbuffer_viewport.maxs = cvmath::Vec2i(width, height);
	instance.resx.update_back(g);
	instance.play.draw(g, &instance.resx, time);
	instance.resx.present(g, time);
	instance.play.metrics = g.get_draw_metrics(true);
}
