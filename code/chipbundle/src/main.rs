use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut args = std::env::args_os().skip(1);
	let output = args.next().ok_or("usage: chipbundle OUTPUT_DIRECTORY")?;
	if args.next().is_some() {
		return Err("usage: chipbundle OUTPUT_DIRECTORY".into());
	}
	let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
	chipbundle::generate_bundled_assets(root, output)?;
	Ok(())
}
