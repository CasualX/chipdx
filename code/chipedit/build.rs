fn main() {
	#[cfg(windows)] {
		let icon_path = "resx/window.ico";
		println!("cargo:rerun-if-changed={icon_path}");

		let mut res = winres::WindowsResource::new();
		res.set_icon(icon_path);
		res.compile().expect("failed to compile Windows resources");
	}
}
