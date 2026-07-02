use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::{fs, io, path};

use cvmath::*;
use shade::image::GridBinPacker;

type Image = shade::image::Image<[u8; 4]>;

mod config;

struct FrameAsset {
	file: String,
	image: Image,
	gutter: config::GutterMode,
}

const SPRITE_FRAME_TIME: f32 = 1.0 / 16.0;

const GUTTER: i32 = 1;

fn area_with_gutter(image: &Image) -> i32 {
	(image.width + GUTTER * 2) * (image.height + GUTTER * 2)
}

fn main() {
	let root = path::Path::new("gfx/MS/");
	let sprite_config = config::File::load(root).sprites;
	let frame_assets = load_unique_frames(root, &sprite_config);
	println!(
		"Loaded {} sprite definitions referencing {} unique files",
		sprite_config.len(),
		frame_assets.len()
	);
	let mut total_area = 0;
	for frame in &frame_assets {
		total_area += area_with_gutter(&frame.image);
		println!(
			"{} -> {}x{}, {} bytes",
			frame.file,
			frame.image.width,
			frame.image.height,
			frame.image.data.len()
		);
	}
	println!("Total sprite area: {} pixels", total_area);
	let sheet_width = 512;
	let sheet_height = 512;
	assert!(total_area <= sheet_width * sheet_height);

	let mut sheet = Image::new(sheet_width, sheet_height, [0; 4]);

	let mut packer = GridBinPacker::new(sheet_width, sheet_height, 32 + GUTTER * 2, 32 + GUTTER * 2);
	let mut packing_order: Vec<usize> = (0..frame_assets.len()).collect();
	packing_order.sort_by_key(|&idx| Reverse(area_with_gutter(&frame_assets[idx].image)));
	let mut packed_frames = 0;
	let mut frame_lookup: HashMap<String, [i32; 4]> = HashMap::new();
	for idx in packing_order {
		let frame = &frame_assets[idx];
		let padded_width = frame.image.width + GUTTER * 2;
		let padded_height = frame.image.height + GUTTER * 2;
		let (x, y) = packer.insert(padded_width, padded_height)
			.unwrap_or_else(|| panic!("sheet too small for {}", frame.file));
		let draw_x = x + GUTTER;
		let draw_y = y + GUTTER;
		let gutter_mode = frame.gutter.into();
		sheet.copy_with_gutter(Point2i(draw_x, draw_y), &frame.image, GUTTER, gutter_mode);
		let rect = [draw_x, draw_y, frame.image.width, frame.image.height];
		frame_lookup.insert(frame.file.clone(), rect);
		packed_frames += 1;
	}

	let mut atlas_sprites = HashMap::new();
	for sprite in &sprite_config {
		let mut frames = Vec::with_capacity(sprite.frames.len());
		for path in &sprite.frames {
			let rect = frame_lookup.get(path)
				.unwrap_or_else(|| panic!("frame {} missing from packed sheet", path));
			let frame = shade::atlas::Frame {
				rect: cvmath::Recti(rect[0], rect[1], rect[2], rect[3]),
				margin: GUTTER,
				transform: sprite.transform,
				origin: sprite_origin(path),
			};
			frames.push(shade::atlas::AnimatedFrame { frame, duration: SPRITE_FRAME_TIME });
		}
		let sprite_entry = match frames.as_slice() {
			[frame] => shade::atlas::Sprite::Frame(frame.frame.clone()),
			_ => shade::atlas::Sprite::Animated(frames),
		};
		atlas_sprites.insert(sprite.name.clone(), sprite_entry);
	}

	sheet.recover_alpha_colors();
	sheet.save_file_png(path::Path::new("data/spritesheet.png")).expect("save spritesheet png");
	let emitted_frames: usize = atlas_sprites.values().map(shade::atlas::Sprite::len).sum();
	let atlas = shade::atlas::Atlas {
		version: 0,
		meta: shade::atlas::Metadata {
			width: sheet_width,
			height: sheet_height,
			kind: shade::atlas::Kind::Bitmap,
			distance_range: 0.0,
			distance_range_middle: 0.0,
		},
		sprites: atlas_sprites,
		fonts: HashMap::new(),
	};
	save_metadata(&atlas, path::Path::new("data/spritesheet.json"));
	println!(
		"Packed {} unique images, emitted {} sprite frames across {} sprites",
		packed_frames,
		emitted_frames,
		sprite_config.len()
	);
}


fn load_unique_frames(root: &path::Path, sprites: &[config::Sprite]) -> Vec<FrameAsset> {
	let mut seen: HashSet<String> = HashSet::new();
	let mut frames = Vec::new();
	for sprite in sprites {
		for rel in &sprite.frames {
			if seen.insert(rel.clone()) {
				let frame_path = root.join(rel);
				let image = shade::image::DecodedImage::load_file_png(frame_path).expect("load sprite frame png").to_rgba();
				let file = rel.clone();
				frames.push(FrameAsset { file, image, gutter: sprite.gutter });
			}
		}
	}
	frames
}

fn sprite_origin(file_name: &str) -> Vec2<i32> {
	if file_name == "BlobH_3.png" {
		return Vec2(16, 0);
	}
	else if file_name == "BlobH_4.png" {
		return Vec2(32, 0);
	}
	else if file_name == "BlobH_5.png" {
		return Vec2(32, 0);
	}
	else if file_name == "BlobV_3.png" {
		return Vec2(0, 16);
	}
	else if file_name == "BlobV_4.png" {
		return Vec2(0, 32);
	}
	else if file_name == "BlobV_5.png" {
		return Vec2(0, 32);
	}
	Vec2::new(0, 0)
}

fn save_metadata(sheet: &shade::atlas::Atlas, path: &path::Path) {
	let file = fs::File::create(path).expect("create spritesheet metadata json");
	let writer = io::BufWriter::new(file);
	serde_json::to_writer(writer, sheet).expect("serialize spritesheet metadata");
}
