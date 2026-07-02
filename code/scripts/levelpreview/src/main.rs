use std::{fs, io, path};

use cvmath::*;
use shade::atlas;

type Image = shade::image::ImageRGBA;

const DEFAULT_TILESET: &str = "data/spritesheet";

fn main() {
	let matches = clap::command!()
		.about("Render a Chip's Challenge level preview to a PNG using software pixel blits")
		.arg(clap::arg!(--json <JSON_PATH> "Path to a level JSON file").value_parser(clap::value_parser!(path::PathBuf)))
		.arg(clap::arg!(--paks <PAKS_PATH> "Path to a packed levelset .paks").value_parser(clap::value_parser!(path::PathBuf)))
		.arg(clap::arg!(--dat <DAT_PATH> "Path to a levelset .dat").value_parser(clap::value_parser!(path::PathBuf)))
		.arg(clap::arg!(-k --key [KEY] "Packed levelset encryption key"))
		.arg(clap::arg!(--tileset [TILESET] "Tileset path without extension, loading .json and .png").value_parser(clap::value_parser!(path::PathBuf)).default_value(DEFAULT_TILESET))
		.arg(clap::arg!(-o --output <OUTPUT_PNG> "Path to output .png").value_parser(clap::value_parser!(path::PathBuf)).required(true))
		.arg(clap::arg!([LEVEL] "Level number (1-based) or case-insensitive level name substring"))
		.get_matches();

	let json = matches.get_one::<path::PathBuf>("json");
	let paks = matches.get_one::<path::PathBuf>("paks");
	let dat = matches.get_one::<path::PathBuf>("dat");
	let output = matches.get_one::<path::PathBuf>("output").unwrap();
	let selector = matches.get_one::<String>("LEVEL");

	let level = match (json, paks, dat) {
		(Some(path), None, None) => {
			if selector.is_some() {
				fail("LEVEL is only used with --paks or --dat");
			}
			load_level_json(path)
		}
		(None, Some(path), None) => {
			let Some(selector) = selector else {
				fail("missing LEVEL number or name for --paks");
			};
			let key = matches.get_one::<String>("key")
				.map(|s| paks::parse_key(s).unwrap_or_else(|err| panic!("invalid key: {err}")))
				.unwrap_or_default();
			load_level_paks(path, &key, selector)
		}
		(None, None, Some(path)) => {
			let Some(selector) = selector else {
				fail("missing LEVEL number or name for --dat");
			};
			load_level_dat(path, selector)
		}
		(None, None, None) => fail("missing --json PATH, --paks PATH, or --dat PATH"),
		_ => fail("use exactly one of --json, --paks, or --dat"),
	};

	let tileset = matches.get_one::<path::PathBuf>("tileset").unwrap();
	let (atlas, sheet) = load_tileset(tileset);
	let preview = render_preview(&level, &atlas, &sheet);
	preview.save_file_png(output).unwrap_or_else(|err| panic!("save {}: {err}", output.display()));
	println!("Wrote {}", output.display());
}

fn fail(message: &str) -> ! {
	eprintln!("levelpreview: {message}");
	std::process::exit(2);
}

fn load_level_json(path: &path::Path) -> chipty::LevelDto {
	let file = fs::File::open(path).unwrap_or_else(|err| panic!("open {}: {err}", path.display()));
	serde_json::from_reader(file).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

fn load_level_paks(path: &path::Path, key: &paks::Key, selector: &str) -> chipty::LevelDto {
	let reader = paks::FileReader::open(path, key).unwrap_or_else(|err| panic!("open {}: {err}", path.display()));
	let desc = reader.find_file(b"index.json").unwrap_or_else(|| panic!("{} does not contain index.json", path.display()));
	let data = reader.read_data(desc, key).unwrap_or_else(|err| panic!("read index.json from {}: {err}", path.display()));
	let index_data = chipty::decompress(&data);
	let levelset: chipty::LevelSetDto = serde_json::from_slice(&index_data).unwrap_or_else(|err| panic!("parse index.json from {}: {err}", path.display()));
	select_level(&levelset, path, selector)
}

fn load_level_dat(path: &path::Path, selector: &str) -> chipty::LevelDto {
	let opts = chipdat::Options {
		encoding: chipdat::Encoding::Windows1252,
	};
	let dat = chipdat::read(path, &opts).unwrap_or_else(|err| panic!("read {}: {err:?}", path.display()));
	let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("levelset").to_string();
	let levelset = chipdat::convert(&dat, title);
	select_level(&levelset, path, selector)
}

fn select_level(levelset: &chipty::LevelSetDto, path: &path::Path, selector: &str) -> chipty::LevelDto {
	if let Ok(number) = selector.parse::<usize>() {
		if number == 0 || number > levelset.levels.len() {
			panic!("level number {number} is outside 1..={}", levelset.levels.len());
		}
		return level_from_ref(&levelset.levels[number - 1], number);
	}

	let selector = selector.to_lowercase();
	for (index, level_ref) in levelset.levels.iter().enumerate() {
		let level = level_from_ref(level_ref, index + 1);
		if level.name.to_lowercase().contains(&selector) {
			return level;
		}
	}

	panic!("level matching {selector:?} not found in {}", path.display());
}

fn level_from_ref(level_ref: &chipty::LevelRef, level_number: usize) -> chipty::LevelDto {
	match level_ref {
		chipty::LevelRef::Direct(level) => level.clone(),
		chipty::LevelRef::Indirect(_) => panic!("level {level_number} is indirect; packed levelsets should embed direct levels"),
	}
}

fn load_tileset(base: &path::Path) -> (atlas::Atlas<chipty::SpriteId>, Image) {
	let json_path = with_extension(base, "json");
	let png_path = with_extension(base, "png");
	let file = fs::File::open(&json_path).unwrap_or_else(|err| panic!("open {}: {err}", json_path.display()));
	let reader = io::BufReader::new(file);
	let atlas = serde_json::from_reader(reader).unwrap_or_else(|err| panic!("parse {}: {err}", json_path.display()));
	let sheet = Image::load_file_png(&png_path).unwrap_or_else(|err| panic!("load {}: {err}", png_path.display()));
	(atlas, sheet)
}

fn with_extension(base: &path::Path, extension: &str) -> path::PathBuf {
	let mut path = base.to_path_buf();
	path.set_extension(extension);
	path
}

fn render_preview(level: &chipty::LevelDto, atlas: &atlas::Atlas<chipty::SpriteId>, sheet: &Image) -> Image {
	let tile_size = sprite_frame(atlas, chipty::SpriteId::Floor).rect.width;
	let width = level.map.width;
	let height = level.map.height;
	let expected_len = (width as i64 * height as i64) as usize;
	if width <= 0 || height <= 0 || level.map.data.len() != expected_len {
		panic!("invalid map dimensions {width}x{height} for {} cells", level.map.data.len());
	}

	let mut preview = Image::new(width * tile_size, height * tile_size, [0, 0, 0, 0]);
	for y in 0..height {
		for x in 0..width {
			let tile = level.map.data[(y * width + x) as usize] as usize;
			let terrain = level.map.legend.get(tile).copied()
				.unwrap_or_else(|| panic!("map cell {x},{y} references missing tile {tile}"));
			blit_sprite(atlas, sheet, &mut preview, terrain_sprite(terrain), Point2i(x, y) * tile_size);
		}
	}

	for entity in &level.entities {
		let sprite = entity_sprite(entity.kind, entity.face_dir);
		blit_sprite(atlas, sheet, &mut preview, sprite, entity.pos * tile_size);
	}

	preview
}

fn terrain_sprite(terrain: chipty::Terrain) -> chipty::SpriteId {
	match terrain {
		chipty::Terrain::Blank => chipty::SpriteId::Blank,
		chipty::Terrain::Floor => chipty::SpriteId::Floor,
		chipty::Terrain::Wall => chipty::SpriteId::Wall,
		chipty::Terrain::Socket => chipty::SpriteId::Floor,
		chipty::Terrain::BlueLock => chipty::SpriteId::BlueLock,
		chipty::Terrain::RedLock => chipty::SpriteId::RedLock,
		chipty::Terrain::GreenLock => chipty::SpriteId::GreenLock,
		chipty::Terrain::YellowLock => chipty::SpriteId::YellowLock,
		chipty::Terrain::Hint => chipty::SpriteId::Hint,
		chipty::Terrain::Exit => chipty::SpriteId::ExitA,
		chipty::Terrain::FakeExit => chipty::SpriteId::ExitA,
		chipty::Terrain::Water => chipty::SpriteId::WaterA,
		chipty::Terrain::WaterHazard => chipty::SpriteId::WaterHazard,
		chipty::Terrain::Fire => chipty::SpriteId::Floor,
		chipty::Terrain::Dirt => chipty::SpriteId::Dirt,
		chipty::Terrain::DirtBlock => chipty::SpriteId::DirtBlock,
		chipty::Terrain::Gravel => chipty::SpriteId::Gravel,
		chipty::Terrain::Ice => chipty::SpriteId::Ice,
		chipty::Terrain::IceNW => chipty::SpriteId::IceCornerNW,
		chipty::Terrain::IceNE => chipty::SpriteId::IceCornerNE,
		chipty::Terrain::IceSW => chipty::SpriteId::IceCornerSW,
		chipty::Terrain::IceSE => chipty::SpriteId::IceCornerSE,
		chipty::Terrain::ForceN => chipty::SpriteId::ForceFloorN,
		chipty::Terrain::ForceW => chipty::SpriteId::ForceFloorW,
		chipty::Terrain::ForceS => chipty::SpriteId::ForceFloorS,
		chipty::Terrain::ForceE => chipty::SpriteId::ForceFloorE,
		chipty::Terrain::ForceRandom => chipty::SpriteId::ForceRandom,
		chipty::Terrain::CloneMachine => chipty::SpriteId::CloneMachine,
		chipty::Terrain::CloneBlockN => chipty::SpriteId::CloneBlockN,
		chipty::Terrain::CloneBlockW => chipty::SpriteId::CloneBlockW,
		chipty::Terrain::CloneBlockS => chipty::SpriteId::CloneBlockS,
		chipty::Terrain::CloneBlockE => chipty::SpriteId::CloneBlockE,
		chipty::Terrain::ToggleFloor => chipty::SpriteId::ToggleFloor,
		chipty::Terrain::ToggleWall => chipty::SpriteId::ToggleWall,
		chipty::Terrain::ThinWallN => chipty::SpriteId::ThinWallN,
		chipty::Terrain::ThinWallW => chipty::SpriteId::ThinWallW,
		chipty::Terrain::ThinWallS => chipty::SpriteId::ThinWallS,
		chipty::Terrain::ThinWallE => chipty::SpriteId::ThinWallE,
		chipty::Terrain::ThinWallNW => chipty::SpriteId::ThinWallNW,
		chipty::Terrain::ThinWallNE => chipty::SpriteId::ThinWallNE,
		chipty::Terrain::ThinWallSW => chipty::SpriteId::ThinWallSW,
		chipty::Terrain::ThinWallSE => chipty::SpriteId::ThinWallSE,
		chipty::Terrain::ThinWallH => chipty::SpriteId::ThinWallH,
		chipty::Terrain::ThinWallV => chipty::SpriteId::ThinWallV,
		chipty::Terrain::HiddenWall => chipty::SpriteId::Floor,
		chipty::Terrain::InvisibleWall => chipty::SpriteId::Floor,
		chipty::Terrain::RealBlueWall => chipty::SpriteId::RealBlueWall,
		chipty::Terrain::FakeBlueWall => chipty::SpriteId::RealBlueWall,
		chipty::Terrain::GreenButton => chipty::SpriteId::GreenButton,
		chipty::Terrain::RedButton => chipty::SpriteId::RedButton,
		chipty::Terrain::BrownButton => chipty::SpriteId::BrownButton,
		chipty::Terrain::BlueButton => chipty::SpriteId::BlueButton,
		chipty::Terrain::Teleport => chipty::SpriteId::Teleport,
		chipty::Terrain::BearTrap => chipty::SpriteId::BearTrap,
		chipty::Terrain::RecessedWall => chipty::SpriteId::RecessedWall,
	}
}

fn entity_sprite(kind: chipty::EntityKind, face_dir: Option<chipty::Compass>) -> chipty::SpriteId {
	match kind {
		chipty::EntityKind::Player => match face_dir.unwrap_or(chipty::Compass::Down) {
			chipty::Compass::Up => chipty::SpriteId::PlayerWalkN,
			chipty::Compass::Left => chipty::SpriteId::PlayerWalkW,
			chipty::Compass::Down => chipty::SpriteId::PlayerWalkS,
			chipty::Compass::Right => chipty::SpriteId::PlayerWalkE,
		},
		chipty::EntityKind::PlayerNPC => chipty::SpriteId::PlayerWalkIdle,
		chipty::EntityKind::Chip => chipty::SpriteId::Chip,
		chipty::EntityKind::Socket => chipty::SpriteId::Socket,
		chipty::EntityKind::Block => chipty::SpriteId::DirtBlock,
		chipty::EntityKind::IceBlock => chipty::SpriteId::IceBlock,
		chipty::EntityKind::Flippers => chipty::SpriteId::Flippers,
		chipty::EntityKind::FireBoots => chipty::SpriteId::FireBoots,
		chipty::EntityKind::IceSkates => chipty::SpriteId::IceSkates,
		chipty::EntityKind::SuctionBoots => chipty::SpriteId::SuctionBoots,
		chipty::EntityKind::BlueKey => chipty::SpriteId::BlueKey,
		chipty::EntityKind::RedKey => chipty::SpriteId::RedKey,
		chipty::EntityKind::GreenKey => chipty::SpriteId::GreenKey,
		chipty::EntityKind::YellowKey => chipty::SpriteId::YellowKey,
		chipty::EntityKind::Thief => chipty::SpriteId::Thief,
		chipty::EntityKind::Bomb => chipty::SpriteId::Bomb,
		chipty::EntityKind::Bug => match face_dir.unwrap_or(chipty::Compass::Up) {
			chipty::Compass::Up => chipty::SpriteId::BugN,
			chipty::Compass::Left => chipty::SpriteId::BugW,
			chipty::Compass::Down => chipty::SpriteId::BugS,
			chipty::Compass::Right => chipty::SpriteId::BugE,
		},
		chipty::EntityKind::FireBall => chipty::SpriteId::Fireball,
		chipty::EntityKind::PinkBall => chipty::SpriteId::PinkBall,
		chipty::EntityKind::Tank => match face_dir.unwrap_or(chipty::Compass::Up) {
			chipty::Compass::Up => chipty::SpriteId::TankN,
			chipty::Compass::Left => chipty::SpriteId::TankW,
			chipty::Compass::Down => chipty::SpriteId::TankS,
			chipty::Compass::Right => chipty::SpriteId::TankE,
		},
		chipty::EntityKind::Glider => match face_dir.unwrap_or(chipty::Compass::Up) {
			chipty::Compass::Up => chipty::SpriteId::GliderN,
			chipty::Compass::Left => chipty::SpriteId::GliderW,
			chipty::Compass::Down => chipty::SpriteId::GliderS,
			chipty::Compass::Right => chipty::SpriteId::GliderE,
		},
		chipty::EntityKind::Teeth => match face_dir.unwrap_or(chipty::Compass::Up) {
			chipty::Compass::Up => chipty::SpriteId::TeethN,
			chipty::Compass::Left => chipty::SpriteId::TeethW,
			chipty::Compass::Down => chipty::SpriteId::TeethS,
			chipty::Compass::Right => chipty::SpriteId::TeethE,
		},
		chipty::EntityKind::Walker => match face_dir.unwrap_or(chipty::Compass::Up) {
			chipty::Compass::Up => chipty::SpriteId::WalkerN,
			chipty::Compass::Left => chipty::SpriteId::WalkerW,
			chipty::Compass::Down => chipty::SpriteId::WalkerS,
			chipty::Compass::Right => chipty::SpriteId::WalkerE,
		},
		chipty::EntityKind::Blob => chipty::SpriteId::Blob,
		chipty::EntityKind::Paramecium => match face_dir.unwrap_or(chipty::Compass::Up) {
			chipty::Compass::Up => chipty::SpriteId::ParameciumN,
			chipty::Compass::Left => chipty::SpriteId::ParameciumW,
			chipty::Compass::Down => chipty::SpriteId::ParameciumS,
			chipty::Compass::Right => chipty::SpriteId::ParameciumE,
		},
	}
}

fn sprite_frame(atlas: &atlas::Atlas<chipty::SpriteId>, sprite: chipty::SpriteId) -> &atlas::Frame {
	atlas.sprites
		.get(&sprite)
		.unwrap_or_else(|| panic!("sprite {:?} missing from atlas", sprite))
		.get_frame_wrapping(0)
		.unwrap_or_else(|| panic!("sprite {:?} has no frames", sprite))
}

fn blit_sprite(atlas: &atlas::Atlas<chipty::SpriteId>, sheet: &Image, preview: &mut Image, sprite: chipty::SpriteId, dst: Point2i) {
	if sprite == chipty::SpriteId::Blank {
		return;
	}
	let frame = sprite_frame(atlas, sprite);
	blit_frame(sheet, preview, frame, dst - frame.origin);
}

fn blit_frame(sheet: &Image, preview: &mut Image, frame: &atlas::Frame, dst: Point2i) {
	if frame.transform == atlas::Transform::None {
		preview.copy_blend(dst, sheet, frame.rect, alpha_over);
		return;
	}

	let rect = cvmath::Rect(dst.x, dst.y, frame.rect.width, frame.rect.height);
	preview.fill_rect(rect, |dst, pos| {
		let src = transform_source(frame, pos);
		let pixel = sheet.read(frame.rect.x + src.x, frame.rect.y + src.y)
			.unwrap_or_else(|| panic!("frame rect {:?} exceeds tileset image bounds", frame.rect));
		alpha_over(dst, pixel)
	});
}

fn transform_source(frame: &atlas::Frame, Point2i { x, y }: Point2i) -> Point2i {
	match frame.transform {
		atlas::Transform::None => Vec2i(x, y),
		atlas::Transform::Rotate90 => Vec2i(y, frame.rect.width - 1 - x),
		atlas::Transform::Rotate180 => Vec2i(frame.rect.width - 1 - x, frame.rect.height - 1 - y),
		atlas::Transform::Rotate270 => Vec2i(frame.rect.height - 1 - y, x),
		atlas::Transform::FlipX => Vec2i(frame.rect.width - 1 - x, y),
		atlas::Transform::FlipSlash => Vec2i(frame.rect.height - 1 - y, frame.rect.width - 1 - x),
		atlas::Transform::FlipY => Vec2i(x, frame.rect.height - 1 - y),
		atlas::Transform::FlipBackslash => Vec2i(y, x),
	}
}

fn alpha_over(dst: [u8; 4], src: [u8; 4]) -> [u8; 4] {
	if src[3] == 0 {
		return dst;
	}
	if src[3] == 255 {
		return src;
	}
	let alpha = src[3] as u32;
	let inv_alpha = 255 - alpha;
	let out_alpha = alpha + (dst[3] as u32 * inv_alpha + 127) / 255;
	let blend = |dst: u8, src: u8| -> u8 {
		((src as u32 * alpha + dst as u32 * inv_alpha + 127) / 255) as u8
	};
	let red = blend(dst[0], src[0]);
	let green = blend(dst[1], src[1]);
	let blue = blend(dst[2], src[2]);
	let alpha = out_alpha.min(255) as u8;
	[red, green, blue, alpha]
}
