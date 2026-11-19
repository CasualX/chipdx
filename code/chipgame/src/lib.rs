
pub mod editor;
pub mod fx;
pub mod menu;
pub mod play;
pub mod config;
pub mod render;

pub enum FileSystem {
	Memory(paks::MemoryReader, paks::Key),
	Paks(paks::FileReader, paks::Key),
	Bundle(paks::BundleReader<'static>),
	StdFs(std::path::PathBuf),
}
impl FileSystem {
	pub fn read_to_string(&self, path: &str) -> std::io::Result<String> {
		match self {
			FileSystem::Memory(paks, key) => {
				let desc = paks.find_file(path.as_bytes()).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
				let data = paks.read_data(&desc, key)?;
				String::from_utf8(data).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))
			}
			FileSystem::Paks(paks, key) => {
				let desc = paks.find_desc(path.as_bytes()).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
				let data = paks.read_data(&desc, key)?;
				String::from_utf8(data).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))
			}
			FileSystem::Bundle(bundle) => {
				let data = bundle.read(path.as_bytes(), bundle.key()).map_err(|_| std::io::Error::from(std::io::ErrorKind::NotFound))?;
				String::from_utf8(data).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))
			}
			FileSystem::StdFs(base) => std::fs::read_to_string(base.join(path)),
		}
	}
	pub fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
		match self {
			FileSystem::Memory(paks, key) => {
				let desc = paks.find_file(path.as_bytes()).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
				let data = paks.read_data(&desc, key)?;
				Ok(data)
			}
			FileSystem::Paks(paks, key) => {
				let desc = paks.find_desc(path.as_bytes()).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
				let data = paks.read_data(&desc, key)?;
				Ok(data)
			}
			FileSystem::Bundle(bundle) => {
				let data = bundle.read(path.as_bytes(), bundle.key()).map_err(|_| std::io::Error::from(std::io::ErrorKind::NotFound))?;
				Ok(data)
			}
			FileSystem::StdFs(base) => std::fs::read(base.join(path)),
		}
	}
	/// Reads and decompresses a file.
	///
	/// Files in a paks archive are zlib compressed.
	pub fn read_compressed(&self, path: &str) -> std::io::Result<Vec<u8>> {
		let data = self.read(path)?;
		if matches!(self, FileSystem::StdFs(_)) {
			Ok(data)
		}
		else {
			chipty::try_decompress(&data)
		}
	}
}
