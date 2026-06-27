
#[link(wasm_import_module = "env")]
extern "C" {
	fn randomBytes(ptr: *mut u8, len: usize);
	pub fn playSound(sound_id: i32);
	pub fn playMusic(music_id: i32);
	pub fn registerSound(sound_id: i32, data_ptr: *const u8, data_len: usize);
	pub fn registerMusic(music_id: i32, data_ptr: *const u8, data_len: usize);
	pub fn setTitle(title_ptr: *const u8, title_len: usize);
	pub fn requestLevelSetFile();
	pub fn resultError(message_ptr: *const u8, message_len: usize);
	pub fn readFile(path_ptr: *const u8, path_len: usize, content_ptr: *mut u8, content_len: *mut usize) -> i32;
	pub fn writeFile(path_ptr: *const u8, path_len: usize, content_ptr: *const u8, content_len: usize) -> i32;
}

pub fn result_error(message: &str) {
	unsafe {
		resultError(message.as_ptr(), message.len());
	}
}

// Support getrandom in wasm builds

#[no_mangle]
unsafe extern "Rust" fn __getrandom_v03_custom(dest: *mut u8, len: usize) -> Result<(), getrandom_03::Error> {
	unsafe { randomBytes(dest, len) };
	Ok(())
}

fn __getrandom_02_custom(buf: &mut [u8]) -> Result<(), getrandom_02::Error> {
	unsafe { randomBytes(buf.as_mut_ptr(), buf.len()) };
	Ok(())
}

getrandom_02::register_custom_getrandom!(__getrandom_02_custom);
