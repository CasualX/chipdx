use super::*;

pub struct DrawScoreCard {
	pub attempts: i32,
	pub time: i32,
	pub steps: i32,
	pub bonks: i32,
	pub time_high_score: i32,
	pub steps_high_score: i32,
}
impl DrawScoreCard {
	pub fn draw<'a>(&self, buf: &mut shade::d2::TextBuffer<'a>, rect: &Bounds2<f32>, resx: &'a Resources) {
		let size = resx.viewport.height() as f32 * FONT_SIZE;

		let mut scribe = shade::d2::Scribe {
			font_size: size,
			line_height: size * (5.0 / 4.0),
			color: Vec4(255, 255, 255, 255),
			..Default::default()
		};

		scribe.color = Vec4(255, 255, 255, 255);
		buf.text_box(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleLeft, "Attempts:\nTime:\nSteps:\nBonks:");

		scribe.color = Vec4(0, 255, 128, 255);
		buf.text_box(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleRight, &format!("{}\n{}\n{}\n{}", self.attempts, chipcore::FmtTime::new(&self.time), self.steps, self.bonks));

		if self.time_high_score < -1 || self.steps_high_score < -1 {
			return;
		}

		let rect = Bounds2!(rect.right(), rect.top(), rect.right(), rect.bottom());
		scribe.color = Vec4(255, 223, 0, 255); // Gold color

		fn status_str(score: i32, high_score: i32) -> impl fmt::Display {
			fmtools::fmt! { move
				if high_score < 0 || score < high_score { " 🏆" }
				else if score == high_score { " 🎯" }
				else { " +"{score - high_score} }
			}
		}

		buf.text_lines(&resx.font, &scribe, &rect, shade::d2::TextAlign::MiddleLeft, &[
			&"",
			&status_str(self.time, self.time_high_score),
			&status_str(self.steps, self.steps_high_score),
			&"",
		]);
	}
}
