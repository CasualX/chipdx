#![cfg(target_family = "unix")]

use std::{mem, ptr, sync};
use std::ffi::*;

use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JValue};
use jni::sys::*;
use jni::{JNIEnv, JavaVM, NativeMethod};

const CHIPDX_INI: &str = include_str!("../../../chipdx.ini");
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const FRAME_NANOS: u64 = NANOS_PER_SECOND / chipcore::FPS as u64;
const JNI_CLASS_NAME: &str = "net/casualhacks/chipdx/ChipJNI";

paks::static_bundle!(DATA_PAK = concat!(env!("OUT_DIR"), "/data.paks"));
paks::static_bundle!(CCLP1_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp1.paks"));
paks::static_bundle!(CCLP2_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp2.paks"));
paks::static_bundle!(CCLP3_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp3.paks"));
paks::static_bundle!(CCLP4_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp4.paks"));
paks::static_bundle!(CCLP5_PAK = concat!(env!("OUT_DIR"), "/levelsets/cclp5.paks"));

#[link(name = "EGL")]
extern "C" {
	fn eglGetProcAddress(name: *const c_char) -> *const c_void;
}

struct HostBridge {
	vm: JavaVM,
	host: GlobalRef,
}

impl HostBridge {
	fn new(env: &mut JNIEnv, host: JObject) -> Result<Self, String> {
		Ok(Self {
			vm: env.get_java_vm().map_err(|err| err.to_string())?,
			host: env.new_global_ref(host).map_err(|err| err.to_string())?,
		})
	}

	fn with_env<T>(&self, f: impl FnOnce(&mut JNIEnv, &JObject) -> Result<T, String>) -> Result<T, String> {
		let mut env = self.vm.attach_current_thread().map_err(|err| err.to_string())?;
		f(&mut env, self.host.as_obj())
	}

	fn set_title(&self, title: &str) {
		let _ = self.with_env(|env, host| {
			let jtitle = env.new_string(title).map_err(|err| err.to_string())?;
			let jtitle = JObject::from(jtitle);
			env.call_method(host, "setNativeTitle", "(Ljava/lang/String;)V", &[JValue::Object(&jtitle)])
				.map_err(|err| err.to_string())?;
			Ok(())
		});
	}

	fn quit_game(&self) {
		let _ = self.with_env(|env, host| {
			env.call_method(host, "quitGame", "()V", &[]).map_err(|err| err.to_string())?;
			Ok(())
		});
	}

	fn play_sound(&self, sound: chipty::SoundFx) {
		let _ = self.with_env(|env, host| {
			env.call_method(host, "playSound", "(I)V", &[JValue::Int(sound as jint)])
				.map_err(|err| err.to_string())?;
			Ok(())
		});
	}

	fn play_music(&self, music: Option<chipty::MusicId>) {
		let id = music.map(|value| value as jint).unwrap_or(-1);
		let _ = self.with_env(|env, host| {
			env.call_method(host, "playMusic", "(I)V", &[JValue::Int(id)])
				.map_err(|err| err.to_string())?;
			Ok(())
		});
	}

	fn register_sound(&self, sound: chipty::SoundFx, path: &str, data: &[u8]) {
		let _ = self.register_audio_asset("registerSound", sound as jint, path, data);
	}

	fn register_music(&self, music: chipty::MusicId, path: &str, data: &[u8]) {
		let _ = self.register_audio_asset("registerMusic", music as jint, path, data);
	}

	fn register_audio_asset(&self, method: &str, id: jint, path: &str, data: &[u8]) -> Result<(), String> {
		self.with_env(|env, host| {
			let jpath = env.new_string(path).map_err(|err| err.to_string())?;
			let jpath = JObject::from(jpath);
			let jdata = env.byte_array_from_slice(data).map_err(|err| err.to_string())?;
			let jdata = JObject::from(jdata);
			env.call_method(
				host,
				method,
				"(ILjava/lang/String;[B)V",
				&[
					JValue::Int(id),
					JValue::Object(&jpath),
					JValue::Object(&jdata),
				],
			)
			.map_err(|err| err.to_string())?;
			Ok(())
		})
	}

	fn write_file(&self, path: &str, content: &[u8]) -> Result<(), String> {
		self.with_env(|env, host| {
			let jpath = env.new_string(path).map_err(|err| err.to_string())?;
			let jpath = JObject::from(jpath);
			let jdata = env.byte_array_from_slice(content).map_err(|err| err.to_string())?;
			let jdata = JObject::from(jdata);
			let result = env
				.call_method(
					host,
					"saveFile",
					"(Ljava/lang/String;[B)Z",
					&[
						JValue::Object(&jpath),
						JValue::Object(&jdata),
					],
				)
				.map_err(|err| err.to_string())?
				.z()
				.map_err(|err| err.to_string())?;
			if result {
				Ok(())
			}
			else {
				Err(format!("Host failed to save {path}"))
			}
		})
	}

	fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>, String> {
		self.with_env(|env, host| {
			let jpath = env.new_string(path).map_err(|err| err.to_string())?;
			let jpath = JObject::from(jpath);
			let result = env
				.call_method(host, "loadFile", "(Ljava/lang/String;)[B", &[JValue::Object(&jpath)])
				.map_err(|err| err.to_string())?
				.l()
				.map_err(|err| err.to_string())?;
			if result.is_null() {
				return Ok(None);
			}
			let array = JByteArray::from(result);
			env.convert_byte_array(array)
				.map(Some)
				.map_err(|err| err.to_string())
		})
	}
}

fn host_slot() -> &'static sync::Mutex<Option<HostBridge>> {
	static HOST: sync::OnceLock<sync::Mutex<Option<HostBridge>>> = sync::OnceLock::new();
	HOST.get_or_init(|| sync::Mutex::new(None))
}

fn with_host<T>(f: impl FnOnce(&HostBridge) -> T) -> Option<T> {
	let guard = host_slot().lock().unwrap();
	guard.as_ref().map(f)
}

fn build_data_fs() -> chipgame::FileSystem {
	let key = paks::Key::default();
	let bundle = paks::BundleReader::open(&DATA_PAK, key).expect("Failed to open bundled data.paks");
	chipgame::FileSystem::Bundle(bundle)
}

fn load_levelset(data: &'static [paks::Block], name: &str, play: &mut chipgame::play::PlayState) {
	let key = paks::Key::default();
	let bundle = paks::BundleReader::open(data, key).expect("Failed to open bundled levelset");
	let fs = chipgame::FileSystem::Bundle(bundle);
	chipgame::play::load_levelset(&fs, name.to_string(), &mut play.lvsets.collection);
}

fn load_curated_levelsets(play: &mut chipgame::play::PlayState) {
	load_levelset(&CCLP1_PAK, "cclp1", play);
	load_levelset(&CCLP2_PAK, "cclp2", play);
	load_levelset(&CCLP3_PAK, "cclp3", play);
	load_levelset(&CCLP4_PAK, "cclp4", play);
	load_levelset(&CCLP5_PAK, "cclp5", play);
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
	with_host(|host| host.set_title(&title));
}

fn register_audio_assets(config: &chipgame::config::Config) {
	let fs = build_data_fs();
	with_host(|host| {
		for (&fx, path) in &config.sound_fx {
			if let Ok(data) = fs.read(path) {
				host.register_sound(fx, path, &data);
			}
		}
		for (&music, path) in &config.music {
			if let Ok(data) = fs.read(path) {
				host.register_music(music, path, &data);
			}
		}
	});
}

fn gl_lib_handle() -> *mut c_void {
	static LIB: sync::OnceLock<usize> = sync::OnceLock::new();
	(*LIB.get_or_init(|| unsafe {
		let name = c"libGLESv3.so";
		let handle = libc::dlopen(name.as_ptr() as *const c_char, libc::RTLD_NOW | libc::RTLD_LOCAL);
		if !handle.is_null() {
			return handle as usize;
		}
		let fallback = c"libGLESv2.so";
		libc::dlopen(fallback.as_ptr() as *const c_char, libc::RTLD_NOW | libc::RTLD_LOCAL) as usize
	})) as *mut c_void
}

fn load_egl(name: &str) -> *const c_void {
	let cname = CString::new(name).expect("GL symbol contains NUL");
	unsafe {
		let symbol = eglGetProcAddress(cname.as_ptr());
		if !symbol.is_null() {
			return symbol;
		}
		let handle = gl_lib_handle();
		if handle.is_null() {
			return ptr::null();
		}
		libc::dlsym(handle, cname.as_ptr()) as *const c_void
	}
}

pub struct Instance {
	resx: Option<chipgame::fx::Resources>,
	graphics: Option<shade::gl::GlGraphics>,
	play: chipgame::play::PlayState,
	config: chipgame::config::Config,
	buttons: u8,
	width: i32,
	height: i32,
	last_frame_nanos: Option<u64>,
	tick_budget_nanos: u64,
	audio_registered: bool,
}

impl Instance {
	fn new() -> Self {
		Self {
			graphics: None,
			resx: None,
			play: chipgame::play::PlayState::default(),
			config: chipgame::config::Config::parse(CHIPDX_INI),
			buttons: 0,
			width: 1,
			height: 1,
			last_frame_nanos: None,
			tick_budget_nanos: FRAME_NANOS / 2,
			audio_registered: false,
		}
	}

	fn rebuild_surface_state(&mut self) {
		shade::gl::capi::load_with(load_egl);

		// GPU resources must be dropped before their owning graphics backend.
		self.resx = None;
		self.graphics = None;

		let fs = build_data_fs();
		let mut graphics = shade::gl::GlGraphics::new(shade::gl::GlConfig { srgb: false });
		let mut resx = chipgame::fx::Resources::load(&fs, &self.config, &mut graphics);
		resx.backbuffer_viewport.maxs = cvmath::Vec2i(self.width.max(1), self.height.max(1));

		let mut play = chipgame::play::PlayState::default();
		load_curated_levelsets(&mut play);
		play.launch(&mut graphics);

		self.graphics = Some(graphics);
		self.resx = Some(resx);
		self.play = play;
		self.last_frame_nanos = None;
		self.tick_budget_nanos = FRAME_NANOS / 2;
		set_title(&self.play);
	}

	fn handle_events(&mut self) {
		let Some(graphics) = self.graphics.as_mut() else { return };
		for evt in mem::take(&mut self.play.events) {
			match evt {
				chipgame::play::PlayEvent::PlaySound { sound } => {
					with_host(|host| host.play_sound(sound));
				}
				chipgame::play::PlayEvent::PlayMusic { music } => {
					with_host(|host| host.play_music(music));
				}
				chipgame::play::PlayEvent::SetTitle => set_title(&self.play),
				chipgame::play::PlayEvent::Restart => self.play.launch(graphics),
				chipgame::play::PlayEvent::LoadExternalLevelSet => {}
				chipgame::play::PlayEvent::Quit => {
					with_host(|host| host.quit_game());
				}
			}
		}
	}
}

fn unsafe_as_instance<'a>(handle: jlong) -> Option<&'a mut Instance> {
	unsafe { (handle as *mut Instance).as_mut() }
}

extern "system" fn native_create(mut env: JNIEnv, _class: JClass, host: JObject) -> jlong {
	match HostBridge::new(&mut env, host) {
		Ok(bridge) => {
			*host_slot().lock().unwrap() = Some(bridge);
			Box::into_raw(Box::new(Instance::new())) as jlong
		}
		Err(err) => {
			eprintln!("Failed to create host bridge: {err}");
			0
		}
	}
}

extern "system" fn native_destroy(_env: JNIEnv, _class: JClass, handle: jlong) {
	if handle != 0 {
		unsafe {
			drop(Box::from_raw(handle as *mut Instance));
		}
	}
	*host_slot().lock().unwrap() = None;
}

extern "system" fn native_audio_init(_env: JNIEnv, _class: JClass, handle: jlong) {
	let Some(instance) = unsafe_as_instance(handle) else { return };
	if instance.audio_registered {
		return;
	}
	register_audio_assets(&instance.config);
	instance.audio_registered = true;
}

extern "system" fn native_on_surface_created(_env: JNIEnv, _class: JClass, handle: jlong) -> jboolean {
	let Some(instance) = unsafe_as_instance(handle) else { return 0 };
	instance.rebuild_surface_state();
	1
}

extern "system" fn native_on_surface_changed(_env: JNIEnv, _class: JClass, handle: jlong, width: jint, height: jint) {
	let Some(instance) = unsafe_as_instance(handle) else { return };
	instance.width = width.max(1);
	instance.height = height.max(1);
	if let Some(resx) = &mut instance.resx {
		resx.backbuffer_viewport.maxs = cvmath::Vec2i(instance.width, instance.height);
	}
}

extern "system" fn native_set_buttons(_env: JNIEnv, _class: JClass, handle: jlong, buttons: jint) {
	let Some(instance) = unsafe_as_instance(handle) else { return };
	instance.buttons = buttons as u8;
}

extern "system" fn native_frame(_env: JNIEnv, _class: JClass, handle: jlong, frame_time_nanos: jlong) {
	let Some(instance) = unsafe_as_instance(handle) else { return };
	if instance.graphics.is_none() || instance.resx.is_none() {
		return;
	}

	let frame_time_nanos = frame_time_nanos.max(0) as u64;
	let frame_dt = match instance.last_frame_nanos.replace(frame_time_nanos) {
		Some(last) => frame_time_nanos.saturating_sub(last),
		None => FRAME_NANOS,
	};
	instance.tick_budget_nanos = instance.tick_budget_nanos.saturating_add(frame_dt.min(FRAME_NANOS * 4));

	let input = chipcore::Input::decode(instance.buttons);
	while instance.tick_budget_nanos >= FRAME_NANOS {
		instance.play.think(&input);
		instance.tick_budget_nanos -= FRAME_NANOS;
	}
	instance.handle_events();

	let graphics = instance.graphics.as_mut().unwrap();
	let resx = instance.resx.as_mut().unwrap();
	resx.backbuffer_viewport.maxs = cvmath::Vec2i(instance.width, instance.height);
	resx.update_back(graphics);
	let time = frame_time_nanos as f64 / NANOS_PER_SECOND as f64;
	instance.play.draw(graphics, resx, time);
	resx.present(graphics, time);
	instance.play.metrics = graphics.get_draw_metrics(true);
}

#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut c_void) -> jint {
	let mut env = match vm.get_env() {
		Ok(env) => env,
		Err(err) => {
			eprintln!("JNI_OnLoad get_env failed: {err}");
			return JNI_ERR;
		}
	};
	let class = match env.find_class(JNI_CLASS_NAME) {
		Ok(class) => class,
		Err(err) => {
			eprintln!("JNI_OnLoad find_class failed for {JNI_CLASS_NAME}: {err}");
			return JNI_ERR;
		}
	};
	let methods = [
		NativeMethod { name: "nativeCreate".into(), sig: "(Ljava/lang/Object;)J".into(), fn_ptr: native_create as *mut c_void },
		NativeMethod { name: "nativeDestroy".into(), sig: "(J)V".into(), fn_ptr: native_destroy as *mut c_void },
		NativeMethod { name: "nativeAudioInit".into(), sig: "(J)V".into(), fn_ptr: native_audio_init as *mut c_void },
		NativeMethod { name: "nativeOnSurfaceCreated".into(), sig: "(J)Z".into(), fn_ptr: native_on_surface_created as *mut c_void },
		NativeMethod { name: "nativeOnSurfaceChanged".into(), sig: "(JII)V".into(), fn_ptr: native_on_surface_changed as *mut c_void },
		NativeMethod { name: "nativeSetButtons".into(), sig: "(JI)V".into(), fn_ptr: native_set_buttons as *mut c_void },
		NativeMethod { name: "nativeFrame".into(), sig: "(JJ)V".into(), fn_ptr: native_frame as *mut c_void },
	];
	if let Err(err) = env.register_native_methods(class, &methods) {
		eprintln!("JNI_OnLoad register_native_methods failed: {err}");
		return JNI_ERR;
	}
	JNI_VERSION_1_6
}

fn unsafe_as_str(ptr: *const u8, len: usize) -> &'static str {
	unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) }
}

#[no_mangle]
extern "C" fn chipgame_write_file(path_ptr: *const u8, path_len: usize, content_ptr: *const u8, content_len: usize) -> i32 {
	let Some(result) = with_host(|host| {
		let path = unsafe_as_str(path_ptr, path_len);
		let content = unsafe { std::slice::from_raw_parts(content_ptr, content_len) };
		host.write_file(path, content)
	}) else {
		return -1;
	};
	if result.is_ok() { 0 } else { -1 }
}

#[no_mangle]
extern "C" fn chipgame_read_file(path_ptr: *const u8, path_len: usize, content_ptr: *mut String) -> i32 {
	let Some(result) = with_host(|host| {
		let path = unsafe_as_str(path_ptr, path_len);
		host.read_file(path)
	}) else {
		return -1;
	};
	let Ok(Some(bytes)) = result else {
		return -1;
	};
	let Ok(content) = String::from_utf8(bytes) else {
		return -1;
	};
	unsafe {
		*content_ptr = content;
	}
	0
}
