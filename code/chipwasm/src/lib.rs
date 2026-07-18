// #![cfg(target_family = "wasm")]

use std::{mem, ptr, str};

mod api;
mod edit;
mod play;

const CHIPDX_INI: &str = include_str!("../../../chipdx.ini");
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

fn create_graphics_resources(
	config: &chipgame::config::Config,
) -> (shade::webgl::WebGLGraphics, chipgame::fx::Resources) {
	let key = paks::Key::default();
	let paks = paks::BundleReader::open(&DATA_PAK, key).expect("Failed to open data.paks");
	let fs = chipgame::FileSystem::Bundle(paks);
	let mut graphics = shade::webgl::WebGLGraphics::new(shade::webgl::WebGLConfig {
		srgb: false,
	});
	let resx = chipgame::fx::Resources::load(&fs, config, &mut graphics);
	(graphics, resx)
}

fn alloc_result_string(value: String, len_ptr: *mut usize) -> *mut u8 {
	if len_ptr.is_null() {
		api::result_error("Missing result length pointer");
		return ptr::null_mut();
	}
	let mut bytes = value.into_bytes().into_boxed_slice();
	let len = bytes.len();
	let out = bytes.as_mut_ptr();
	mem::forget(bytes);
	unsafe {
		*len_ptr = len;
	}
	out
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

#[no_mangle]
extern "C" fn chipgame_write_file(path_ptr: *const u8, path_len: usize, content_ptr: *const u8, content_len: usize) -> i32 {
	unsafe { api::writeFile(path_ptr, path_len, content_ptr, content_len) }
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
