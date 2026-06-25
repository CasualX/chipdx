use super::*;

#[repr(transparent)]
struct FmtTrophyLine(Option<chipty::Trophy>);
impl fmt::Display for FmtTrophyLine {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let &FmtTrophyLine(trophy) = self;

		f.write_str(match trophy {
			Some(chipty::Trophy::Author) => "\x1b[color=#8B5CF6]🏆\x1b[color=fff]\n",
			Some(chipty::Trophy::Gold) => "\x1b[color=#FFD700]🏆\x1b[color=fff]\n",
			Some(chipty::Trophy::Silver) => "\x1b[color=#C9CDD1]🏆\x1b[color=fff]\n",
			Some(chipty::Trophy::Bronze) => "\x1b[color=#CD7F32]🏆\x1b[color=fff]\n",
			None => "\n",
		})
	}
}

struct FmtTrophiesLeft(DrawTrophies);
impl FmtTrophiesLeft {
	fn new(this: &DrawTrophies) -> &FmtTrophiesLeft {
		unsafe { mem::transmute(this) }
	}
}
impl fmt::Display for FmtTrophiesLeft {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let FmtTrophiesLeft(this) = self;

		write!(f, "Level {}:\n\x1b[color=#ff0]{}\x1b[color=#fff]\n", this.level_number, this.level_name)?;
		if let Some(author) = &this.level_author {
			write!(f, "By \x1b[color=#00FFD1]{}\x1b[color=#fff]\n", author)?;
		}
		f.write_str("\nAttempts:\n")?;
		if this.tick.high_score >= 0 {
			f.write_str("Best time:\n")?;
		}
		if this.steps.high_score >= 0 {
			f.write_str("Least steps:\n")?;
		}
		f.write_str("\n")?;
		if let Some((next_time, _)) = &this.tick.next {
			write!(f, "{} time:\n", next_time.to_str())?;
		}
		if let Some((next_steps, _)) = &this.steps.next {
			write!(f, "{} steps:\n", next_steps.to_str())?;
		}
		Ok(())
	}
}

#[repr(transparent)]
struct FmtTrophiesRight(DrawTrophies);
impl FmtTrophiesRight {
	fn new(this: &DrawTrophies) -> &FmtTrophiesRight {
		unsafe { mem::transmute(this) }
	}
}
impl fmt::Display for FmtTrophiesRight {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let FmtTrophiesRight(this) = self;

		if this.level_author.is_some() {
			f.write_str("\n")?;
		}
		write!(f, "\n\n\n{}\n", this.attempts)?;
		if this.tick.high_score >= 0 {
			write!(f, "{}\n", chipcore::FmtTime::new(&this.tick.high_score))?;
		}
		if this.steps.high_score >= 0 {
			write!(f, "{}\n", this.steps.high_score)?;
		}
		f.write_str("\n")?;
		if let Some((_, next_value)) = &this.tick.next {
			write!(f, "{}\n", chipcore::FmtTime::new(next_value))?;
		}
		if let Some((_, next_value)) = &this.steps.next {
			write!(f, "{}\n", *next_value)?;
		}

		Ok(())
	}
}

#[repr(transparent)]
struct FmtTrophiesTrophies(DrawTrophies);
impl FmtTrophiesTrophies {
	fn new(this: &DrawTrophies) -> &FmtTrophiesTrophies {
		unsafe { mem::transmute(this) }
	}
}
impl fmt::Display for FmtTrophiesTrophies {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let FmtTrophiesTrophies(this) = self;

		if this.level_author.is_some() {
			f.write_str("\n")?;
		}
		write!(f, "\n\n\n\n{}{}\n{}{}",
			FmtTrophyLine(this.tick.trophy),
			FmtTrophyLine(this.steps.trophy),
			FmtTrophyLine(this.tick.next.as_ref().map(|t| t.0)),
			FmtTrophyLine(this.steps.next.as_ref().map(|t| t.0)))
	}
}

struct DrawTrophyValues {
	high_score: i32,
	trophy: Option<chipty::Trophy>,
	next: Option<(chipty::Trophy, i32)>,
}
impl DrawTrophyValues {
	fn new(high_score: i32, values: Option<&chipty::TrophyValues>) -> DrawTrophyValues {
		let Some(values) = values else {
			return DrawTrophyValues { high_score, trophy: None, next: None };
		};

		let trophy = if high_score < 0 { None }
		else if high_score <= values.author && values.author >= 0 { Some(chipty::Trophy::Author) }
		else if high_score <= values.gold && values.gold >= 0 { Some(chipty::Trophy::Gold) }
		else if high_score <= values.silver && values.silver >= 0 { Some(chipty::Trophy::Silver) }
		else if high_score <= values.bronze && values.bronze >= 0 { Some(chipty::Trophy::Bronze) }
		else { None };

		let next = if values.bronze >= 0 && high_score > values.bronze {
			Some((chipty::Trophy::Bronze, values.bronze))
		}
		else if values.silver >= 0 && high_score > values.silver {
			Some((chipty::Trophy::Silver, values.silver))
		}
		else if values.gold >= 0 && high_score > values.gold {
			Some((chipty::Trophy::Gold, values.gold))
		}
		else if values.author >= 0 && high_score > values.author {
			Some((chipty::Trophy::Author, values.author))
		}
		else {
			None
		};
		DrawTrophyValues { high_score, trophy, next }
	}
}

pub struct DrawTrophies {
	level_number: i32,
	level_name: String,
	level_author: Option<String>,
	attempts: i32,
	tick: DrawTrophyValues,
	steps: DrawTrophyValues,
}
impl DrawTrophies {
	pub fn new(level_number: i32, level: &chipty::LevelDto, save_data: &play::SaveData) -> DrawTrophies {
		let tick_high_score = save_data.get_time_high_score(level_number);
		let steps_high_score = save_data.get_steps_high_score(level_number);
		let tick_values = level.trophies.as_ref().map(|m| &m.ticks);
		let steps_values = level.trophies.as_ref().map(|m| &m.steps);
		let attempts = save_data.get_attempts(level_number);
		DrawTrophies {
			level_number,
			level_name: level.name.clone(),
			level_author: level.author.clone(),
			attempts,
			tick: DrawTrophyValues::new(tick_high_score, tick_values),
			steps: DrawTrophyValues::new(steps_high_score, steps_values),
		}
	}
	pub fn draw<'a>(&self, buf: &mut shade::d2::TextBuffer<'a>, panel: &Bounds2<f32>, resx: &'a Resources) {
		let size = resx.font_size();

		let mut scribe = shade::d2::Scribe {
			font_size: size * 0.75,
			line_height: size * 0.75 / 32.0 * 40.0,
			color: Vec4(255, 255, 255, 255),
			..Default::default()
		};

		let left_str = FmtTrophiesLeft::new(self).to_string();
		let right_str = FmtTrophiesRight::new(self).to_string();
		let trophy_str = FmtTrophiesTrophies::new(self).to_string();

		let [left, _, right] = draw::flexh(*panel, None, layout::Justify::Center, &[layout::Unit::Fr(2.0), layout::Unit::Abs(size * 0.5), layout::Unit::Fr(1.0)]);
		buf.text_box(&resx.font, &scribe, &left, shade::d2::TextAlign::TopLeft, &left_str);
		scribe.color = Vec4(0, 255, 128, 255);
		buf.text_box(&resx.font, &scribe, &left, shade::d2::TextAlign::TopRight, &right_str);
		buf.text_box(&resx.font, &scribe, &right, shade::d2::TextAlign::TopLeft, &trophy_str);
	}
}
