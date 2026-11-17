use std::{collections::HashMap, fs, io, path};

use super::{atlas, Image};

const FINAL_HOLD_TICKS: u64 = 4 * chipcore::FPS as u64;

// One palette for the entire replay avoids color flicker and per-frame tables.
// Index zero is reserved for unchanged pixels in delta frames. Flatten alpha
// onto black, which also lets later frames erase sprites on blank terrain.
struct Palette {
	colors: Vec<u8>,
	cache: HashMap<[u8; 3], u8>,
}

impl Palette {
	fn new(sheet: &Image) -> Self {
		let rgb: Vec<u8> = sheet.data.iter().flat_map(|&pixel| Self::rgb(pixel)).collect();
		let frame = gif::Frame::from_rgb_speed(sheet.width as u16, sheet.height as u16, &rgb, 10);
		let mut colors = vec![0, 0, 0];
		colors.extend(frame.palette.unwrap().into_iter().take(255 * 3));
		Self { colors, cache: HashMap::new() }
	}

	fn rgb(pixel: [u8; 4]) -> [u8; 3] {
		let a = pixel[3] as u32;
		[0, 1, 2].map(|i| ((pixel[i] as u32 * a + 127) / 255) as u8)
	}

	fn index(&mut self, image: &Image) -> Vec<u8> {
		image.data.iter().map(|&pixel| {
			let rgb = Self::rgb(pixel);
			*self.cache.entry(rgb).or_insert_with(|| {
				self.colors.chunks_exact(3).enumerate().skip(1)
					.min_by_key(|(_, color)| (0..3).map(|i| (rgb[i] as i32 - color[i] as i32).pow(2)).sum::<i32>())
					.unwrap().0 as u8
			})
		}).collect()
	}
}

struct DeltaEncoder<W: io::Write> {
	encoder: gif::Encoder<W>,
	width: u16,
	height: u16,
	previous: Vec<u8>,
	pending: Option<gif::Frame<'static>>,
	ticks: u64,
	written_time: u64,
	frames: usize,
}

impl<W: io::Write> DeltaEncoder<W> {
	fn new(writer: W, width: u16, height: u16, palette: &[u8]) -> Result<Self, gif::EncodingError> {
		let mut encoder = gif::Encoder::new(writer, width, height, palette)?;
		encoder.set_repeat(gif::Repeat::Infinite)?;
		Ok(Self { encoder, width, height, previous: Vec::new(), pending: None, ticks: 0, written_time: 0, frames: 0 })
	}

	fn push(&mut self, pixels: Vec<u8>) -> Result<(), gif::EncodingError> {
		assert_eq!(pixels.len(), self.width as usize * self.height as usize);
		// Bound merged delays to the GIF's 16-bit centisecond field.
		if pixels == self.previous && self.time() - self.written_time < 65_000 {
			self.ticks += 1;
			return Ok(());
		}
		self.flush()?;
		let width = self.width as usize;
		let mut left = width;
		let mut top = self.height as usize;
		let mut right = 0;
		let mut bottom = 0;
		for (i, &pixel) in pixels.iter().enumerate() {
			if self.previous.get(i) == Some(&pixel) { continue; }
			left = left.min(i % width);
			right = right.max(i % width);
			top = top.min(i / width);
			bottom = bottom.max(i / width);
		}
		// A long unchanged hold needs another frame to carry its remaining delay.
		if left == width { left = 0; top = 0; }
		let mut buffer = Vec::new();
		for y in top..=bottom {
			for x in left..=right {
				let i = y * width + x;
				buffer.push(if self.previous.get(i) == Some(&pixels[i]) { 0 } else { pixels[i] });
			}
		}
		let mut frame = gif::Frame::from_indexed_pixels((right - left + 1) as u16, (bottom - top + 1) as u16, buffer, Some(0));
		frame.left = left as u16;
		frame.top = top as u16;
		frame.dispose = gif::DisposalMethod::Keep;
		self.pending = Some(frame);
		self.previous = pixels;
		self.ticks += 1;
		Ok(())
	}

	fn time(&self) -> u64 {
		(self.ticks * 100 + chipcore::FPS as u64 / 2) / chipcore::FPS as u64
	}

	fn flush(&mut self) -> Result<(), gif::EncodingError> {
		let time = self.time();
		if let Some(mut frame) = self.pending.take() {
			frame.delay = (time - self.written_time) as u16;
			self.encoder.write_frame(&frame)?;
			self.frames += 1;
			self.written_time = time;
		}
		Ok(())
	}
}

pub(super) fn save(
	output: &path::Path, level: &chipty::LevelDto, replay: &chipty::ReplayDto,
	atlas: &atlas::Atlas<chipty::SpriteId>, sheet: &Image, reveal: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	let seed = u64::from_str_radix(&replay.seed, 16).map_err(|err| format!("invalid replay seed: {err}"))?;
	let inputs = chipty::try_decode(&replay.inputs).ok_or("invalid replay inputs")?;
	let mut game = chipcore::GameState::default();
	game.parse(level, chipcore::RngSeed::Manual(seed));
	let initial = super::render_state(level, &game, atlas, sheet, reveal);
	let width = u16::try_from(initial.width)?;
	let height = u16::try_from(initial.height)?;
	let mut palette = Palette::new(sheet);
	let file = io::BufWriter::new(fs::File::create(output)?);
	let mut encoder = DeltaEncoder::new(file, width, height, &palette.colors)?;
	let mut ticks = 0;
	let mut outcome = None;
	for byte in inputs {
		if game.is_game_over() { break; }
		game.tick(&chipcore::Input::decode(byte));
		ticks += 1;
		for event in game.events.take() {
			if let chipcore::GameEvent::GameOver { reason } = event { outcome = Some(reason); }
		}
		let image = super::render_state(level, &game, atlas, sheet, reveal);
		encoder.push(palette.index(&image))?;
	}
	if ticks == 0 { encoder.push(palette.index(&initial))?; }
	// Hold the completed board before looping. Merged delays are capped at
	// 65,000 centiseconds, leaving room for this extra four seconds.
	encoder.ticks += FINAL_HOLD_TICKS;
	encoder.flush()?;
	let frames = encoder.frames;
	// Finish the trailer and report any buffered I/O errors before success.
	let mut writer = encoder.encoder.into_inner()?;
	io::Write::flush(&mut writer)?;
	println!("Replay GIF: {ticks} ticks, {frames} encoded frames, {} bytes, {}",
		fs::metadata(output)?.len(), outcome.map(|reason| format!("{reason:?}")).unwrap_or_else(|| "inputs ended before game over".into()));
	Ok(())
}
