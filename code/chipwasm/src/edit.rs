use super::*;

pub struct EditorInstance {
	resx: chipgame::fx::Resources,
	graphics: shade::webgl::WebGLGraphics,
	editor: chipgame::editor::EditorState,
	music_enabled: bool,
	current_music: Option<chipty::MusicId>,
}

fn create_instance() -> Box<EditorInstance> {
	let mut config = chipgame::config::Config::parse(CHIPDX_INI);
	config.render_scale = 1.0;
	config.post_process = chipgame::config::PostProcess::None;
	let (graphics, resx) = create_graphics_resources(&config);
	let mut editor = chipgame::editor::EditorState::new(include_str!("../../chipedit/src/template.json"));
	editor.set_screen_size(800, 600);
	Box::new(EditorInstance {
		graphics,
		resx,
		editor,
		music_enabled: true,
		current_music: None,
	})
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn createEditorInstance() -> *mut EditorInstance {
	shade::webgl::setup_panic_hook();
	Box::into_raw(create_instance())
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn destroyEditorInstance(instance: *mut EditorInstance) {
	if instance.is_null() {
		return;
	}
	_ = unsafe { Box::from_raw(instance) };
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn thinkEditorInstance(instance: *mut EditorInstance) {
	if instance.is_null() {
		return;
	}
	let instance = unsafe { &mut *instance };
	instance.editor.think();

	for evt in instance.editor.take_fx_events() {
		match evt {
			chipgame::fx::FxEvent::PlaySound(sound) => play_sound(sound),
			chipgame::fx::FxEvent::LevelComplete => {
				instance.editor.save_replay();
				instance.editor.toggle_play();
			}
			chipgame::fx::FxEvent::GameOver => instance.editor.toggle_play(),
			_ => {}
		}
	}

	let music = instance.editor.get_music(instance.music_enabled);
	if music != instance.current_music {
		instance.current_music = music;
		play_music(music);
	}
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn drawEditorInstance(instance: *mut EditorInstance, time: f64, width: i32, height: i32) {
	if instance.is_null() {
		return;
	}
	let instance = unsafe { &mut *instance };
	let g = instance.graphics.as_graphics();
	instance.resx.backbuffer_viewport.maxs = cvmath::Vec2i(width, height);
	instance.resx.update_back(g);
	instance.editor.set_screen_size(width, height);
	instance.editor.draw(g, &instance.resx, time);
	instance.resx.present(g, time);
	_ = g.get_draw_metrics(true);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn isEditorPlaying(instance: *const EditorInstance) -> bool {
	if instance.is_null() {
		return false;
	}
	let instance = unsafe { &*instance };
	instance.editor.is_playing()
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn loadEditorLevel(instance: *mut EditorInstance, level_ptr: *const u8, level_len: usize) -> i32 {
	if instance.is_null() || level_ptr.is_null() {
		api::result_error("Missing editor level payload");
		return -1;
	}
	let json_bytes = unsafe { std::slice::from_raw_parts(level_ptr, level_len) };
	let Ok(json) = str::from_utf8(json_bytes) else {
		api::result_error("Editor level is not valid UTF-8");
		return -1;
	};
	if let Err(err) = serde_json::from_str::<chipty::LevelDto>(json) {
		api::result_error(&format!("Editor level JSON is invalid: {err}"));
		return -1;
	}
	let instance = unsafe { &mut *instance };
	instance.editor = chipgame::editor::EditorState::new(json);
	return 0;
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn saveEditorLevel(instance: *mut EditorInstance, len_ptr: *mut usize) -> *mut u8 {
	if instance.is_null() {
		api::result_error("Missing editor instance");
		return ptr::null_mut();
	}
	let instance = unsafe { &mut *instance };
	alloc_result_string(instance.editor.save_level(), len_ptr)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn shareEditorLevel(instance: *mut EditorInstance, len_ptr: *mut usize) -> *mut u8 {
	if instance.is_null() {
		api::result_error("Missing editor instance");
		return ptr::null_mut();
	}
	let instance = unsafe { &mut *instance };
	let mut level: chipty::LevelDto = match serde_json::from_str(&instance.editor.save_level()) {
		Ok(level) => level,
		Err(err) => {
			api::result_error(&format!("Editor level JSON is invalid: {err}"));
			return ptr::null_mut();
		}
	};
	level.replays = None;
	let Ok(level_json) = serde_json::to_string(&level) else {
		api::result_error("Editor level could not be serialized");
		return ptr::null_mut();
	};
	alloc_result_string(chipty::encode_level(level_json.as_bytes()), len_ptr)
}

fn editor_string_arg(value_ptr: *const u8, value_len: usize) -> Option<String> {
	if value_ptr.is_null() {
		return None;
	}
	let value = unsafe { std::slice::from_raw_parts(value_ptr, value_len) };
	str::from_utf8(value).ok().map(str::to_string)
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn setEditorTextField(instance: *mut EditorInstance, field: i32, value_ptr: *const u8, value_len: usize) -> i32 {
	if instance.is_null() {
		api::result_error("Missing editor instance");
		return -1;
	}
	let Some(value) = editor_string_arg(value_ptr, value_len) else {
		api::result_error("Editor text field value is not valid UTF-8");
		return -1;
	};
	let instance = unsafe { &mut *instance };
	match &mut instance.editor {
		chipgame::editor::EditorState::Edit(editor) => match field {
			0 => editor.set_level_name(value),
			1 => editor.set_author(if value.trim().is_empty() { None } else { Some(value) }),
			2 => editor.set_hint(if value.trim().is_empty() { None } else { Some(value) }),
			_ => {
				api::result_error("Unknown editor text field");
				return -1;
			}
		},
		chipgame::editor::EditorState::Play(_) => return -1,
	}
	return 0;
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn setEditorNumberField(instance: *mut EditorInstance, field: i32, value: i32) -> i32 {
	if instance.is_null() {
		api::result_error("Missing editor instance");
		return -1;
	}
	let instance = unsafe { &mut *instance };
	match &mut instance.editor {
		chipgame::editor::EditorState::Edit(editor) => match field {
			0 => editor.set_required_chips(value),
			1 => editor.set_time_limit(value),
			_ => {
				api::result_error("Unknown editor number field");
				return -1;
			}
		},
		chipgame::editor::EditorState::Play(_) => return -1,
	}
	return 0;
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn setEditorMouse(instance: *mut EditorInstance, x: i32, y: i32) {
	if instance.is_null() {
		return;
	}
	let instance = unsafe { &mut *instance };
	instance.editor.mouse_move(x, y);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn setEditorMouseButton(instance: *mut EditorInstance, button: i32, pressed: bool) {
	if instance.is_null() {
		return;
	}
	let instance = unsafe { &mut *instance };
	match button {
		0 => instance.editor.left_click(pressed),
		1 => instance.editor.right_click(pressed),
		_ => {}
	}
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn setEditorKey(instance: *mut EditorInstance, key: i32, pressed: bool) {
	if instance.is_null() {
		return;
	}
	let instance = unsafe { &mut *instance };
	match key {
		0 => instance.editor.key_left(pressed),
		1 => instance.editor.key_right(pressed),
		2 => instance.editor.key_up(pressed),
		3 => instance.editor.key_down(pressed),
		4 => instance.editor.key_shift(pressed),
		_ => {}
	}
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn panEditorView(instance: *mut EditorInstance, delta_x: f32, delta_y: f32) {
	if instance.is_null() {
		return;
	}
	let instance = unsafe { &mut *instance };
	instance.editor.pan_view(delta_x, delta_y);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn editorCommand(instance: *mut EditorInstance, command: i32) -> i32 {
	if instance.is_null() {
		return -1;
	}
	let instance = unsafe { &mut *instance };
	match command {
		0 => instance.editor.toggle_play(),
		1 => instance.editor.delete(true),
		2 => instance.editor.delete(false),
		3 => instance.editor.undo(),
		4 => instance.editor.redo(),
		5 => instance.editor.tool_terrain(true),
		6 => instance.editor.tool_entity(true),
		7 => instance.editor.tool_connection(true),
		8 => instance.editor.tool_icepath(true),
		9 => instance.editor.tool_forcepath(true),
		10 => instance.editor.tool_entorder(true),
		11 => instance.editor.expand_top(),
		12 => instance.editor.expand_bottom(),
		13 => instance.editor.expand_left(),
		14 => instance.editor.expand_right(),
		15 => instance.editor.crop_top(),
		16 => instance.editor.crop_bottom(),
		17 => instance.editor.crop_left(),
		18 => instance.editor.crop_right(),
		19 => instance.editor.zoom_in(),
		20 => instance.editor.zoom_out(),
		21 => instance.music_enabled = !instance.music_enabled,
		22 => instance.editor.sample(),
		23 => instance.editor.cancel_left_click(),
		24 => instance.editor.right_click(true),
		25 => instance.editor.right_click(false),
		_ => return -1,
	}
	return 0;
}
