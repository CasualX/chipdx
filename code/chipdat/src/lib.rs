use std::{fs, path};

mod convert;
pub use self::convert::convert;

#[derive(Copy, Clone, Debug, Default)]
pub enum Encoding {
	Utf8,
	Latin1,
	#[default]
	Windows1252,
}

#[derive(Clone, Debug)]
pub struct Data {
	pub magic: u32,
	pub levels: Vec<Level>,
}

impl Data {
	pub const MSCC: u32 = 0x0002AAAC;
	pub const LYNX: u32 = 0x0102AAAC;
	pub const PGCHIP: u32 = 0x0003AAAC;
}

#[derive(Clone, Debug)]
pub struct TrapLinkage {
	pub brown_button_x: u16,
	pub brown_button_y: u16,
	pub trap_x: u16,
	pub trap_y: u16,
	pub zero: u16,
}

#[derive(Clone, Debug)]
pub struct ClonerLinkage {
	pub red_button_x: u16,
	pub red_button_y: u16,
	pub cloner_x: u16,
	pub cloner_y: u16,
}

#[derive(Clone, Debug)]
pub struct Monster {
	pub x: u8,
	pub y: u8,
}

#[derive(Clone, Debug, Default)]
pub struct Metadata {
	/// Time limit, in seconds.
	///
	/// Unused, see [Level::time_limit].
	pub time_limit: Option<u16>,

	/// Number of chips required to complete the level.
	///
	/// Unused, see [Level::required_chips].
	pub required_chips: Option<u16>,

	/// Level title.
	///
	/// Required.
	pub title: Option<String>,

	pub traps: Option<Vec<TrapLinkage>>,

	pub cloners: Option<Vec<ClonerLinkage>>,

	/// Level password.
	///
	/// Required.
	pub password: Option<String>,

	pub hint: Option<String>,

	/// Unused, see [Metadata::password].
	pub unencrypted_password: Option<String>,

	pub monsters: Option<Vec<Monster>>,

	pub author: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Level {
	/// Level number.
	pub number: u16,

	/// Time limit, in seconds.
	///
	/// Zero indicates the level is untimed.
	///
	/// Every official version of the game has a three-digit timer so values over 999 are uncommon.
	pub time_limit: u16,

	/// Number of chips required to complete the level.
	///
	/// This may be less than the total number of chips in the level, in which case the extra chips are optional.
	///
	/// This may be more than the total number of chips in the level, in which case the level may be impossible to complete.
	pub required_chips: u16,

	/// Top layer of tiles.
	///
	/// In reading order (left to right, top to bottom).
	pub top_layer: Vec<u8>,

	/// Bottom layer of tiles.
	///
	/// In reading order (left to right, top to bottom).
	pub bottom_layer: Vec<u8>,

	/// Metadata chunks.
	pub metadata: Metadata,
}

#[derive(Debug)]
pub enum DatError {
	/// The file is not a valid .dat file.
	InvalidDatFile,
	/// The level data is invalid.
	InvalidLevelData { index: u16 },
	/// The metadata chunk type is unknown.
	UnknownMetadataChunkType { index: u16, ty: u8 },
	/// The string data is invalid.
	InvalidStringData { index: u16 },
	/// I/O error.
	Io(std::io::Error),
}

pub struct Options {
	pub encoding: Encoding,
}

pub fn read(path: impl AsRef<path::Path>, opts: &Options) -> Result<Data, DatError> {
	let data = fs::read(path).map_err(DatError::Io)?;
	parse(&data, opts)
}

pub fn parse(data: &[u8], opts: &Options) -> Result<Data, DatError> {
	if data.len() < 6 {
		return Err(DatError::InvalidDatFile);
	}

	let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
	let nlevels = u16::from_le_bytes([data[4], data[5]]);

	if !(magic == Data::MSCC || magic == Data::LYNX || magic == Data::PGCHIP) {
		return Err(DatError::InvalidDatFile);
	}

	let mut levels = Vec::new();
	let mut offset = 6;
	for index in 0..nlevels {
		if offset + 2 > data.len() {
			return Err(DatError::InvalidLevelData { index });
		}
		let level_len_in_bytes = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
		if offset + 2 + level_len_in_bytes > data.len() {
			return Err(DatError::InvalidLevelData { index });
		}
		let level_bytes = &data[offset..offset + 2 + level_len_in_bytes];
		offset += 2 + level_len_in_bytes;

		let level = parse_level(index, level_bytes, opts)?;
		levels.push(level);
	}

	Ok(Data { magic, levels })
}

fn parse_level(index: u16, data: &[u8], opts: &Options) -> Result<Level, DatError> {
	if data.len() < 12 {
		return Err(DatError::InvalidLevelData { index });
	}

	let _size = u16::from_le_bytes([data[0], data[1]]) as usize;
	let number = u16::from_le_bytes([data[2], data[3]]);
	let time_limit = u16::from_le_bytes([data[4], data[5]]);
	let required_chips = u16::from_le_bytes([data[6], data[7]]);
	let _unclear = u16::from_le_bytes([data[8], data[9]]);

	let (top_layer, rest) = split_sized_chunk(index, &data[10..])?;
	let (bottom_layer, rest) = split_sized_chunk(index, rest)?;
	let (metadata_bytes, _) = split_sized_chunk(index, rest)?;
	let top_layer = decode_layer_rle(top_layer);
	let bottom_layer = decode_layer_rle(bottom_layer);
	if top_layer.len() < 32 * 32 || bottom_layer.len() < 32 * 32 {
		return Err(DatError::InvalidLevelData { index });
	}
	let metadata = parse_metadata(index, metadata_bytes, opts)?;

	Ok(Level {
		number,
		time_limit,
		required_chips,
		top_layer,
		bottom_layer,
		metadata,
	})
}

fn split_sized_chunk(index: u16, data: &[u8]) -> Result<(&[u8], &[u8]), DatError> {
	let Some((&[lo, hi], data)) = data.split_first_chunk::<2>() else {
		return Err(DatError::InvalidLevelData { index });
	};
	let len = u16::from_le_bytes([lo, hi]) as usize;
	data.split_at_checked(len).ok_or(DatError::InvalidLevelData { index })
}

fn decode_layer_rle(data: &[u8]) -> Vec<u8> {
	// A layer is a simple stream of bytes representing the tiles in reading order.
	// A lone byte indicates a particular tile according to the encoding below.
	// A $FF byte indicates simple run-length encoding, where the next two bytes are the number of copies and the byte to repeat.
	// The sequence `00 FF 0A 01 00` thus describes twelve tiles: a floor tile, a span of ten walls, and another floor tile.

	let mut decoded = Vec::new();
	let mut i = 0;
	while i < data.len() {
		let byte = data[i];
		if byte == 0xFF {
			if i + 2 >= data.len() {
				// Invalid RLE data, not enough bytes for count and value
				break;
			}
			let count = data[i + 1];
			let value = data[i + 2];
			for _ in 0..count {
				decoded.push(value);
			}
			i += 3;
		}
		else {
			decoded.push(byte);
			i += 1;
		}
	}
	decoded
}

fn parse_metadata(index: u16, data: &[u8], opts: &Options) -> Result<Metadata, DatError> {
	let mut metadata = Metadata::default();

	let mut i = 0;
	while i < data.len() {
		let Some(&[ty, len]) = data[i..].first_chunk::<2>() else {
			return Err(DatError::InvalidLevelData { index });
		};
		let len = len as usize;
		let chunk = data.get(i + 2..i + 2 + len).ok_or(DatError::InvalidLevelData { index })?;

		match ty {
			1 => metadata.time_limit = Some(parse_u16(index, chunk)?),
			2 => metadata.required_chips = Some(parse_u16(index, chunk)?),
			3 => metadata.title = Some(parse_string(index, chunk, opts)?),
			4 => metadata.traps = Some(parse_trap_linkage(index, chunk)?),
			5 => metadata.cloners = Some(parse_cloner_linkage(index, chunk)?),
			6 => metadata.password = Some(parse_encrypted_password(index, chunk, opts)?),
			7 => metadata.hint = Some(parse_string(index, chunk, opts)?),
			8 => metadata.unencrypted_password = Some(parse_string(index, chunk, opts)?),
			9 => metadata.author = Some(parse_string(index, chunk, opts)?),
			10 => metadata.monsters = Some(parse_monster_list(index, chunk)?),
			_ => return Err(DatError::UnknownMetadataChunkType { index, ty }),
		}

		i = i + 2 + len;
	}

	Ok(metadata)
}

fn parse_u16(index: u16, data: &[u8]) -> Result<u16, DatError> {
	if data.len() != 2 {
		return Err(DatError::InvalidLevelData { index });
	}
	Ok(u16::from_le_bytes([data[0], data[1]]))
}

fn parse_trap_linkage(index: u16, data: &[u8]) -> Result<Vec<TrapLinkage>, DatError> {
	if data.len() % 10 != 0 {
		return Err(DatError::InvalidLevelData { index });
	}
	let linkages = data
		.chunks_exact(10)
		.map(|chunk| {
			let brown_button_x = u16::from_le_bytes([chunk[0], chunk[1]]);
			let brown_button_y = u16::from_le_bytes([chunk[2], chunk[3]]);
			let trap_x = u16::from_le_bytes([chunk[4], chunk[5]]);
			let trap_y = u16::from_le_bytes([chunk[6], chunk[7]]);
			let zero = u16::from_le_bytes([chunk[8], chunk[9]]);
			TrapLinkage { brown_button_x, brown_button_y, trap_x, trap_y, zero }
		})
		.collect();
	Ok(linkages)
}

fn parse_cloner_linkage(index: u16, data: &[u8]) -> Result<Vec<ClonerLinkage>, DatError> {
	if data.len() % 8 != 0 {
		return Err(DatError::InvalidLevelData { index });
	}
	let linkages = data
		.chunks_exact(8)
		.map(|chunk| {
			let red_button_x = u16::from_le_bytes([chunk[0], chunk[1]]);
			let red_button_y = u16::from_le_bytes([chunk[2], chunk[3]]);
			let cloner_x = u16::from_le_bytes([chunk[4], chunk[5]]);
			let cloner_y = u16::from_le_bytes([chunk[6], chunk[7]]);
			ClonerLinkage { red_button_x, red_button_y, cloner_x, cloner_y }
		})
		.collect();
	Ok(linkages)
}

fn parse_monster_list(index: u16, data: &[u8]) -> Result<Vec<Monster>, DatError> {
	if data.len() % 2 != 0 {
		return Err(DatError::InvalidLevelData { index });
	}
	let monsters = data
		.chunks_exact(2)
		.map(|chunk| Monster { x: chunk[0], y: chunk[1] })
		.collect();
	Ok(monsters)
}

fn parse_encrypted_password(index: u16, data: &[u8], opts: &Options) -> Result<String, DatError> {
	if data.len() == 0 {
		return Err(DatError::InvalidStringData { index });
	}
	let mut bytes = data.to_vec();
	for i in 0..bytes.len() - 1 {
		bytes[i] ^= 0x99;
	}
	parse_string(index, &bytes, opts)
}

fn parse_string(index: u16, data: &[u8], opts: &Options) -> Result<String, DatError> {
	// C string, must be nul terminated and not have nul bytes inside
	let bytes = data.strip_suffix(&[0]).ok_or(DatError::InvalidStringData { index })?;
	if bytes.contains(&0) {
		return Err(DatError::InvalidStringData { index });
	}

	let string = match opts.encoding {
		Encoding::Utf8 => {
			String::from_utf8_lossy(bytes).into_owned()
		}
		Encoding::Latin1 => {
			bytes.iter().map(|&b| b as char).collect()
		}
		Encoding::Windows1252 => {
			bytes.iter().map(|&b| if b < 0x80 { b as char } else { WINDOWS_1252[(b - 0x80) as usize] }).collect()
		}
	};
	Ok(string)
}

static WINDOWS_1252: [char; 128] = [
	'\u{20AC}', // 80
	'\u{FFFD}', // 81 (undefined)
	'\u{201A}', // 82
	'\u{0192}', // 83
	'\u{201E}', // 84
	'\u{2026}', // 85
	'\u{2020}', // 86
	'\u{2021}', // 87
	'\u{02C6}', // 88
	'\u{2030}', // 89
	'\u{0160}', // 8A
	'\u{2039}', // 8B
	'\u{0152}', // 8C
	'\u{FFFD}', // 8D (undefined)
	'\u{017D}', // 8E
	'\u{FFFD}', // 8F (undefined)
	'\u{FFFD}', // 90 (undefined)
	'\u{2018}', // 91
	'\u{2019}', // 92
	'\u{201C}', // 93
	'\u{201D}', // 94
	'\u{2022}', // 95
	'\u{2013}', // 96
	'\u{2014}', // 97
	'\u{02DC}', // 98
	'\u{2122}', // 99
	'\u{0161}', // 9A
	'\u{203A}', // 9B
	'\u{0153}', // 9C
	'\u{FFFD}', // 9D (undefined)
	'\u{017E}', // 9E
	'\u{0178}', // 9F
	// 0xA0 - 0xFF (identical to ISO-8859-1)
	'\u{00A0}', // A0
	'\u{00A1}', // A1
	'\u{00A2}', // A2
	'\u{00A3}', // A3
	'\u{00A4}', // A4
	'\u{00A5}', // A5
	'\u{00A6}', // A6
	'\u{00A7}', // A7
	'\u{00A8}', // A8
	'\u{00A9}', // A9
	'\u{00AA}', // AA
	'\u{00AB}', // AB
	'\u{00AC}', // AC
	'\u{00AD}', // AD
	'\u{00AE}', // AE
	'\u{00AF}', // AF
	'\u{00B0}', // B0
	'\u{00B1}', // B1
	'\u{00B2}', // B2
	'\u{00B3}', // B3
	'\u{00B4}', // B4
	'\u{00B5}', // B5
	'\u{00B6}', // B6
	'\u{00B7}', // B7
	'\u{00B8}', // B8
	'\u{00B9}', // B9
	'\u{00BA}', // BA
	'\u{00BB}', // BB
	'\u{00BC}', // BC
	'\u{00BD}', // BD
	'\u{00BE}', // BE
	'\u{00BF}', // BF
	'\u{00C0}', // C0
	'\u{00C1}', // C1
	'\u{00C2}', // C2
	'\u{00C3}', // C3
	'\u{00C4}', // C4
	'\u{00C5}', // C5
	'\u{00C6}', // C6
	'\u{00C7}', // C7
	'\u{00C8}', // C8
	'\u{00C9}', // C9
	'\u{00CA}', // CA
	'\u{00CB}', // CB
	'\u{00CC}', // CC
	'\u{00CD}', // CD
	'\u{00CE}', // CE
	'\u{00CF}', // CF
	'\u{00D0}', // D0
	'\u{00D1}', // D1
	'\u{00D2}', // D2
	'\u{00D3}', // D3
	'\u{00D4}', // D4
	'\u{00D5}', // D5
	'\u{00D6}', // D6
	'\u{00D7}', // D7
	'\u{00D8}', // D8
	'\u{00D9}', // D9
	'\u{00DA}', // DA
	'\u{00DB}', // DB
	'\u{00DC}', // DC
	'\u{00DD}', // DD
	'\u{00DE}', // DE
	'\u{00DF}', // DF
	'\u{00E0}', // E0
	'\u{00E1}', // E1
	'\u{00E2}', // E2
	'\u{00E3}', // E3
	'\u{00E4}', // E4
	'\u{00E5}', // E5
	'\u{00E6}', // E6
	'\u{00E7}', // E7
	'\u{00E8}', // E8
	'\u{00E9}', // E9
	'\u{00EA}', // EA
	'\u{00EB}', // EB
	'\u{00EC}', // EC
	'\u{00ED}', // ED
	'\u{00EE}', // EE
	'\u{00EF}', // EF
	'\u{00F0}', // F0
	'\u{00F1}', // F1
	'\u{00F2}', // F2
	'\u{00F3}', // F3
	'\u{00F4}', // F4
	'\u{00F5}', // F5
	'\u{00F6}', // F6
	'\u{00F7}', // F7
	'\u{00F8}', // F8
	'\u{00F9}', // F9
	'\u{00FA}', // FA
	'\u{00FB}', // FB
	'\u{00FC}', // FC
	'\u{00FD}', // FD
	'\u{00FE}', // FE
	'\u{00FF}', // FF
];
