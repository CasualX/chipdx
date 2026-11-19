use super::*;

const BUILTIN_LEVELSETS: &[(&[paks::Block], &str)] = &[
	(&CCLP1_PAK, "cclp1"),
	(&CCLP2_PAK, "cclp2"),
	(&CCLP3_PAK, "cclp3"),
	(&CCLP4_PAK, "cclp4"),
	(&CCLP5_PAK, "cclp5"),
];

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
	instance.play.launch(&mut instance.graphics);
	set_title(&instance.play);
}

fn create_instance() -> Box<PlayInstance> {
	let mut config = chipgame::config::Config::parse(CHIPDX_INI);
	config.render_scale = 0.5;
	let (graphics, resx) = create_graphics_resources(&config);
	let play = chipgame::play::PlayState::default();
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

	for &(data, name) in BUILTIN_LEVELSETS {
		load_levelset(data, name.to_string(), &mut instance.play);
	}

	Box::into_raw(instance)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn launchPlayInstance(instance: *mut PlayInstance) {
	let instance = unsafe { &mut *instance };
	instance.play.launch(&mut instance.graphics);
}

fn load_custom_level(mut level: chipty::LevelDto, replay_index: Option<usize>) -> Box<PlayInstance> {
	level.normalize();
	let level_set = chipgame::play::LevelSet {
		name: "Custom Level".to_string(),
		title: "Custom Level".to_string(),
		about: None,
		splash: None,
		unlock_all_levels: false,
		levels: vec![level],
	};

	let mut instance = create_instance();
	instance.play.save_data.ephemeral = true;
	instance.play.load_single_level(level_set);
	instance.play.play_replay(1, replay_index);
	instance
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn createCustomPlayLevel(level_ptr: *const u8, level_len: usize, compressed: bool, replay_index: i32) -> *mut PlayInstance {
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

	let replay_index = if replay_index < 0 { None } else { Some(replay_index as usize) };
	let instance = load_custom_level(level, replay_index);
	Box::into_raw(instance)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn loadPlayLevelSet(instance: *mut PlayInstance, data_ptr: *const u8, data_len: usize, name_ptr: *const u8, name_len: usize) -> i32 {
	if data_ptr.is_null() || name_ptr.is_null() {
		api::result_error("Missing levelset file data or filename");
		return -1;
	}

	let data = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };
	let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
	let Ok(filename) = str::from_utf8(name_bytes) else {
		api::result_error("The levelset filename is invalid");
		return -1;
	};
	let levelset = match read_custom_levelset(data, filename) {
		Ok(levelset) => levelset,
		Err(err) => {
			api::result_error(&err);
			return -1;
		}
	};

	// A null instance validates a file before the browser saves it.
	if let Some(instance) = unsafe { instance.as_mut() } {
		let collection = &mut instance.play.lvsets.collection;
		if collection.iter().any(|set| set.name == levelset.name) {
			api::result_error("A levelset with this filename is already loaded");
			return -1;
		}
		collection.push(levelset);
	}
	return 0;
}

fn read_custom_levelset(data: &[u8], filename: &str) -> Result<chipgame::play::LevelSet, String> {
	use chipgame::play::{levelset_from_dto, read_levelset_index};

	if filename.contains(['/', '\\']) {
		return Err("The levelset filename is invalid".to_string());
	}
	let (stem, extension) = filename.rsplit_once('.').unwrap_or((filename, ""));
	let title = match stem.trim() {
		"" => "Local Levelset",
		title => title,
	};
	let levelset = match extension.to_ascii_lowercase().as_str() {
		"paks" => {
			let key = paks::Key::default();
			let reader = paks::MemoryReader::from_bytes(data, &key).map_err(|_| "Could not open the .paks archive with the default key")?;
			let fs = chipgame::FileSystem::Memory(reader, key);
			let index = read_levelset_index(&fs).map_err(|err| format!("Could not read the .paks levelset index: {err}"))?;
			let level_count = index.levels.len();
			let levelset = levelset_from_dto(Some(&fs), index, format!("custom/{title}.paks"));
			if levelset.levels.len() != level_count {
				return Err("The .paks archive contains unreadable levels".to_string());
			}
			levelset
		}
		"dat" | "ccl" => {
			use chipdat::DatError;

			let opts = chipdat::Options { encoding: chipdat::Encoding::Windows1252 };
			let dat = chipdat::parse(data, &opts).map_err(|err| match err {
				DatError::InvalidDatFile => "The file header does not match a CC1 levelset".to_string(),
				DatError::InvalidLevelData { index } => format!("Level {} contains invalid data", u32::from(index) + 1),
				DatError::UnknownMetadataChunkType { index, .. } => format!("Level {} contains unsupported metadata", u32::from(index) + 1),
				DatError::InvalidStringData { index } => format!("Level {} contains invalid text", u32::from(index) + 1),
				DatError::Io(err) => format!("Could not read the CC1 levelset: {err}"),
			})?;
			let index = chipdat::convert(&dat, title.to_string());
			// Keep the save names used by previously imported CC1 levelsets.
			let name = if BUILTIN_LEVELSETS.iter().any(|&(_, name)| name == title) {
				format!("custom/{title}.dat")
			}
			else {
				title.to_string()
			};
			levelset_from_dto(None, index, name)
		}
		_ => return Err("Choose a .dat, .ccl, or .paks file".to_string()),
	};
	if levelset.levels.is_empty() {
		return Err("The file did not contain any levels".to_string());
	}
	for level in &levelset.levels {
		let map = &level.map;
		if !(chipty::FIELD_MIN_WIDTH..=chipty::FIELD_MAX_WIDTH).contains(&map.width)
			|| !(chipty::FIELD_MIN_HEIGHT..=chipty::FIELD_MAX_HEIGHT).contains(&map.height)
			|| (!map.data.is_empty() && map.data.len() != (map.width * map.height) as usize)
			|| map.data.iter().any(|&tile| tile as usize >= map.legend.len())
		{
			return Err(format!("The map for \"{}\" is invalid", level.name));
		}
	}
	Ok(levelset)
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
			&chipgame::play::PlayEvent::Restart => instance.play.launch(&mut instance.graphics),
			&chipgame::play::PlayEvent::Quit => quit_game(instance),
		}
	}
}

#[no_mangle]
pub extern "C" fn drawPlayInstance(instance: *mut PlayInstance, time: f64, width: i32, height: i32) {
	let instance = unsafe { &mut *instance };
	instance.resx.backbuffer_viewport.maxs = cvmath::Vec2i(width, height);
	instance.resx.update_back(&mut instance.graphics);
	instance.play.draw(&mut instance.graphics, &instance.resx, time);
	instance.resx.present(&mut instance.graphics, time);
	instance.play.metrics = instance.graphics.get_draw_metrics(true);
}
