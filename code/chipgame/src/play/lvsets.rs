use super::*;

#[derive(Default)]
pub struct LevelSet {
	pub name: String,
	pub title: String,
	pub about: Option<String>,
	pub splash: Option<Vec<u8>>,
	pub levels: Vec<chipty::LevelDto>,
}
impl LevelSet {
	pub fn get_level_number(&self, name: &str) -> Option<i32> {
		self.levels.iter().position(|level| level.name == name).map(|i| i as i32 + 1)
	}
	pub fn get_level_index(&self, name: &str) -> Option<usize> {
		self.levels.iter().position(|level| level.name == name)
	}
}

pub struct LevelSets {
	pub selected: i32,
	pub collection: Vec<LevelSet>,
	pub external_loader_label: Option<String>,
}
impl Default for LevelSets {
	fn default() -> Self {
		Self {
			selected: -1,
			collection: Vec::new(),
			external_loader_label: None,
		}
	}
}
impl LevelSets {
	pub fn current(&self) -> &LevelSet {
		&self.collection[self.selected as usize]
	}
	pub fn load(&mut self) {
		load_levelsets(&mut self.collection);
	}
}

fn load_levelsets(sets: &mut Vec<LevelSet>) {
	let dir = match fs::read_dir("levelsets") {
		Ok(dir) => dir,
		Err(err) => {
			eprintln!("Error reading levelsets directory: {}", err);
			return;
		}
	};
	for entry in dir {
		match entry {
			Ok(entry) => {
				let path = entry.path();
				if path.is_dir() {
					let fs = FileSystem::StdFs(path.clone());
					let name = path.file_name().unwrap().to_string_lossy().into_owned();
					load_levelset(&fs, name, sets);
				}
				// Check for packed files if no directory by that name exist
				else if path.is_file() && !path.with_extension("").exists() {
					let Some(ext) = path.extension() else { continue; };
					if ext == "paks" {
						let key_name = format!("chipgame_{}", path.file_stem().unwrap().to_string_lossy());
						let key = match std::env::var(key_name) {
							Ok(val) => {
								match paks::parse_key(&val) {
								Ok(key) => key,
								Err(err) => {
									eprintln!("Invalid key: {err}: {}", path.display());
									continue;
								}
							}},
							Err(_) => paks::Key::default(),
						};
						match paks::FileReader::open(&path, &key) {
							Ok(paks) => {
								let fs = FileSystem::Paks(paks, key);
								let name = path.file_stem().unwrap().to_string_lossy().into_owned();
								load_levelset(&fs, name, sets);
							},
							Err(err) => {
								eprintln!("Error reading {}: {}", path.display(), err);
							}
						};
					}
					else if ext == "dat" {
						load_dat(&path, sets);
					}
				}
			}
			Err(err) => {
				eprintln!("Error reading set: {}", err);
			}
		}
	}
	sets.sort_by(|a, b| a.name.cmp(&b.name));
}

pub fn load_levelset(fs: &FileSystem, name: String, sets: &mut Vec<LevelSet>) {
	let index: chipty::LevelSetDto = {
		let index = match fs.read_compressed("index.json") {
			Ok(data) => data,
			Err(err) => {
				eprintln!("Error reading index.json: {}", err);
				return;
			}
		};
		match serde_json::from_slice(&index) {
			Ok(level_set) => level_set,
			Err(err) => {
				eprintln!("Error parsing index.json: {}", err);
				return;
			}
		}
	};
	load_levelset_dto(Some(fs), index, name, sets);
}

pub fn load_levelset_dto(fs: Option<&FileSystem>, index: chipty::LevelSetDto, name: String, sets: &mut Vec<LevelSet>) {
	let mut levels = Vec::new();
	for level_ref in index.levels {
		let level = match level_ref {
			chipty::LevelRef::Direct(level) => level,
			chipty::LevelRef::Indirect(level_path) => {
				let content = match fs.unwrap().read_to_string(&level_path) {
					Ok(data) => data,
					Err(err) => {
						eprintln!("Error reading {level_path}: {err}");
						continue;
					}
				};
				match serde_json::from_str(&content) {
					Ok(level) => level,
					Err(err) => {
						eprintln!("Error parsing level at {level_path}: {err}");
						continue;
					}
				}
			}
		};
		levels.push(level);
	}

	let splash = index.splash.and_then(|s| match fs? {
		FileSystem::Memory(paks, key) => paks.read(s.as_bytes(), key).ok(),
		FileSystem::Paks(paks, key) => paks.read(s.as_bytes(), key).ok(),
		FileSystem::Bundle(bundle) => bundle.read(s.as_bytes(), bundle.key()).ok(),
		FileSystem::StdFs(path) => fs::read(path.join(s)).ok(),
	});

	let title = index.title;
	let about = index.about.map(|lines| lines.join("\n"));
	sets.push(LevelSet { name, title, about, splash, levels });
}

fn load_dat(path: &path::PathBuf, sets: &mut Vec<LevelSet>) {
	let opts = chipdat::Options {
		encoding: chipdat::Encoding::Windows1252,
	};
	let dat = match chipdat::read(path, &opts) {
		Ok(dat) => dat,
		Err(err) => {
			eprintln!("Error reading {}: {:?}", path.display(), err);
			return;
		}
	};

	let title = path.file_stem().map(|s| String::from_utf8_lossy(s.as_encoded_bytes()).into_owned()).unwrap_or(String::new());

	let index = chipdat::convert(&dat, title.clone());

	load_levelset_dto(None, index, title, sets);
}
