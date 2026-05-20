use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chipty::{LevelDto, LevelRef, LevelSetDto};

pub const CURATED_BUNDLED_LEVELSETS: &[&str] = &["cclp1", "cclp2", "cclp3", "cclp4", "cclp5"];
pub const DATA_PAK_FILENAME: &str = "data.paks";
pub const LEVELSETS_DIRNAME: &str = "levelsets";

pub fn generate_bundled_assets(repo_root: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> io::Result<()> {
	let repo_root = repo_root.as_ref();
	let output_dir = output_dir.as_ref();
	fs::create_dir_all(output_dir)?;
	fs::create_dir_all(output_dir.join(LEVELSETS_DIRNAME))?;

	generate_data_bundle(repo_root.join("data"), output_dir.join(DATA_PAK_FILENAME))?;
	generate_curated_levelset_bundles(repo_root.join("levelsets"), output_dir.join(LEVELSETS_DIRNAME))?;
	Ok(())
}

pub fn generate_data_bundle(data_dir: impl AsRef<Path>, output_file: impl AsRef<Path>) -> io::Result<()> {
	let data_dir = data_dir.as_ref();
	let output_file = output_file.as_ref();
	let key = paks::Key::default();
	let mut editor = create_bundle(output_file, &key)?;

	for file_path in collect_files(data_dir)? {
		let pak_path = relative_pak_path(data_dir, &file_path)?;
		let data = fs::read(&file_path)?;
		editor.create_file(pak_path.as_bytes(), &data, &key)?;
	}

	editor.finish(&key)
}

pub fn generate_curated_levelset_bundles(levelsets_root: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> io::Result<()> {
	let levelsets_root = levelsets_root.as_ref();
	let output_dir = output_dir.as_ref();
	fs::create_dir_all(output_dir)?;

	for levelset in CURATED_BUNDLED_LEVELSETS {
		let input_dir = levelsets_root.join(levelset);
		let output_file = output_dir.join(format!("{levelset}.paks"));
		generate_levelset_bundle(&input_dir, &output_file)?;
	}

	Ok(())
}

pub fn generate_levelset_bundle(levelset_dir: impl AsRef<Path>, output_file: impl AsRef<Path>) -> io::Result<()> {
	let levelset_dir = levelset_dir.as_ref();
	let output_file = output_file.as_ref();
	let key = paks::Key::default();
	let mut levelset = read_levelset(levelset_dir.join("index.json"))?;

	for level in &mut levelset.levels {
		if let LevelRef::Indirect(path) = level {
			let level_data: LevelDto = read_json(levelset_dir.join(path))?;
			*level = LevelRef::Direct(level_data);
		}
	}

	let index_json = serde_json::to_string(&levelset).map_err(io_invalid_data)?;
	let compressed_index = chipty::compress(index_json.as_bytes());
	let mut editor = create_bundle(output_file, &key)?;
	editor.create_file(b"index.json", &compressed_index, &key)?;

	if let Some(splash) = &levelset.splash {
		let splash_data = fs::read(levelset_dir.join(splash))?;
		editor.create_file(splash.as_bytes(), &splash_data, &key)?;
	}

	editor.finish(&key)
}

fn create_bundle(output_file: &Path, key: &paks::Key) -> io::Result<paks::FileEditor> {
	if let Some(parent) = output_file.parent() {
		fs::create_dir_all(parent)?;
	}
	match fs::remove_file(output_file) {
		Ok(()) => {}
		Err(err) if err.kind() == io::ErrorKind::NotFound => {}
		Err(err) => return Err(err),
	}
	paks::FileEditor::create_new(output_file, key)
}

fn collect_files(root: &Path) -> io::Result<Vec<PathBuf>> {
	let mut files = Vec::new();
	collect_files_into(root, &mut files)?;
	files.sort();
	Ok(files)
}

fn collect_files_into(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
	let mut entries: Vec<_> = fs::read_dir(root)?.collect::<Result<_, _>>()?;
	entries.sort_by_key(|entry| entry.file_name());

	for entry in entries {
		let path = entry.path();
		if entry.file_type()?.is_dir() {
			collect_files_into(&path, files)?;
		}
		else {
			files.push(path);
		}
	}

	Ok(())
}

fn relative_pak_path(root: &Path, path: &Path) -> io::Result<String> {
	let relative = path.strip_prefix(root).map_err(io_invalid_input)?;
	let mut pak_path = String::new();
	for component in relative.components() {
		if !pak_path.is_empty() {
			pak_path.push('/');
		}
		pak_path.push_str(component.as_os_str().to_string_lossy().as_ref());
	}
	Ok(pak_path)
}

fn read_levelset(path: PathBuf) -> io::Result<LevelSetDto> {
	read_json(path)
}

fn read_json<T: serde::de::DeserializeOwned>(path: PathBuf) -> io::Result<T> {
	let file = fs::File::open(path)?;
	serde_json::from_reader(file).map_err(io_invalid_data)
}

fn io_invalid_data(err: impl std::error::Error + Send + Sync + 'static) -> io::Error {
	io::Error::new(io::ErrorKind::InvalidData, err)
}

fn io_invalid_input(err: impl std::error::Error + Send + Sync + 'static) -> io::Error {
	io::Error::new(io::ErrorKind::InvalidInput, err)
}
