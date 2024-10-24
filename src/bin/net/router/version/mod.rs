use fontdue::Font;

use crate::net::assets::GameSiteAssets;

/// Generate version png
pub fn gen_version_png(v: &str, w: u32, h: u32) -> Option<Vec<u8>> {
	// Read the font data.
	let font = GameSiteAssets::get("emukc/fonts/Anton-Regular.ttf")?;
	let font = font.data;
	// Parse it into the font type.
	let font = fontdue::Font::from_bytes(font, fontdue::FontSettings::default()).unwrap();

	// calculate font size
	fn measure_text(font: &Font, text: &str, font_size: f32) -> (f32, f32, f32, f32) {
		let mut width = 0.0;
		let mut max_ascent: f32 = 0.0;
		let mut max_descent: f32 = 0.0;

		for c in text.chars() {
			let (metrics, _) = font.rasterize(c, font_size);
			width += metrics.advance_width;

			let bounds = metrics.bounds;
			let bounds_ymax = bounds.ymin + bounds.height;
			let ascent = bounds_ymax;
			let descent = -bounds.ymin;
			max_ascent = max_ascent.max(ascent);
			max_descent = max_descent.max(descent);
		}

		let height = max_ascent + max_descent;
		(width, height, max_ascent, max_descent)
	}

	let mut lower = 1.0;
	let mut upper = h as f32;
	let mut best_size = lower;
	let max_iterations = 20;
	let tolerance = 0.1;

	let (total_width, total_height, _, _) = measure_text(&font, v, upper);
	if total_width < w as f32 && total_height < h as f32 {
		best_size = upper;
	} else {
		for _ in 0..max_iterations {
			let mid = (lower + upper) / 2.0;
			let (text_width, text_height, _, _) = measure_text(&font, v, mid);

			if text_width > w as f32 || text_height > h as f32 {
				upper = mid;
			} else {
				best_size = mid;
				lower = mid;
			}

			if (upper - lower) < tolerance {
				break;
			}
		}
	}

	let font_size = best_size;

	// final measure to get the image size
	let (text_width, text_height, max_ascent, _) = measure_text(&font, v, font_size);

	// create image buffer
	let width = w as usize;
	let height = h as usize;
	let mut img = vec![0u8; width * height * 4];

	let baseline = ((h as f32 - text_height) / 2.0 + max_ascent).round() as isize;
	let text_x = ((w as f32 - text_width) / 2.0).max(0.0) as usize;
	let mut x = text_x;

	for c in v.chars() {
		let (metrics, bitmap) = font.rasterize(c, font_size);

		let glyph_width = metrics.width;
		let glyph_height = metrics.height;

		let bounds_ymax = metrics.ymin as f32 + metrics.height as f32;
		let y_offset_f32 = baseline as f32 - bounds_ymax;
		let y_offset = y_offset_f32.round() as isize;

		for row in 0..glyph_height {
			for col in 0..glyph_width {
				let pixel = bitmap[row * glyph_width + col];
				let img_x = x + col;
				let img_y = y_offset + row as isize;

				if img_x < width && img_y >= 0 && (img_y as usize) < height {
					let img_y = img_y as usize;
					let idx = (img_y * width + img_x) * 4;

					let alpha = pixel;

					if alpha > 0 {
						img[idx] = 75; // R
						img[idx + 1] = 72; // G
						img[idx + 2] = 68; // B
						img[idx + 3] = alpha; // A
					}
				}
			}
		}

		x += metrics.advance_width as usize;
	}

	let mut png_bytes = Vec::new();
	{
		let mut encoder = png::Encoder::new(&mut png_bytes, width as u32, height as u32);
		encoder.set_color(png::ColorType::Rgba);
		encoder.set_depth(png::BitDepth::Eight);
		let mut writer = encoder.write_header().ok()?;
		writer.write_image_data(&img).ok()?;
	}

	Some(png_bytes)
}

#[cfg(test)]
mod test {
	use super::gen_version_png;

	#[test]
	fn test_font() {
		let text = "EmuKC 0.1.0-750AECE";
		let width = 157;
		let height = 27;

		match gen_version_png(text, width, height) {
			Some(png_data) => {
				let size = png_data.len();
				std::fs::write("output.png", png_data).expect("cannot write file");
				println!("png size: {}", size);
			}
			None => {
				println!("failed to generate version png");
			}
		}
	}

	#[test]
	fn test_img_type() {
		let v = "resources/world/localhost_t.png";
		let typ = v.strip_suffix(".png").and_then(|s| s.chars().last());
		println!("{:?}", typ);
	}
}
