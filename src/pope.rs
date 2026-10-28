use std::sync::OnceLock;

use image::{ImageFormat, RgbImage, imageops::FilterType};

use crate::color::{Palette, Rgb};
use crate::grid::{Cell, Grid};
use crate::rng::Rng;

/// "Pope John Paul II smile.jpg", Wikimedia Commons, CC BY 3.0 pl.
static JPG: &[u8] = include_bytes!("../assets/jp2.jpg");

/// Rampy od "pustego" do "pełnego" znaku; przy każdej przemianie losujemy jedną.
const RAMPS: &[&str] = &[
    " .:-=+*#%@",
    " .,:;irsXA253hMHGS#9B&@",
    " .'`^\",:;Il!i><~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$",
    " .:;+=xX$&",
];

fn photo() -> &'static RgbImage {
    static PHOTO: OnceLock<RgbImage> = OnceLock::new();
    PHOTO.get_or_init(|| {
        image::load_from_memory_with_format(JPG, ImageFormat::Jpeg)
            .expect("wbudowane zdjęcie jest poprawnym JPEG-iem")
            .to_rgb8()
    })
}

/// Kadr głowa + ramiona — w ASCII liczy się twarz, nie cała sutanna.
fn portrait() -> &'static RgbImage {
    static PORTRAIT: OnceLock<RgbImage> = OnceLock::new();
    PORTRAIT.get_or_init(|| {
        let img = photo();
        let (w, h) = (img.width() as f32, img.height() as f32);
        let (x, y) = ((w * 0.12) as u32, (h * 0.06) as u32);
        let (cw, ch) = ((w * 0.76) as u32, (h * 0.62) as u32);
        image::imageops::crop_imm(img, x, y, cw, ch).to_image()
    })
}

pub fn luminance(c: Rgb) -> f32 {
    (0.2126 * c.0 as f32 + 0.7152 * c.1 as f32 + 0.0722 * c.2 as f32) / 255.0
}

/// Papież z ASCII. Komórki poza portretem są puste (spacja na tle terminala).
/// Gęstość znaku odpowiada jasności zdjęcia — na ciemnym tle jasna sutanna
/// to `@`, na jasnym odwrotnie.
pub fn ascii(w: u16, h: u16, pal: &Palette, rng: &mut Rng) -> Grid {
    let img = portrait();
    // Komórka terminala jest ~2× wyższa niż szersza.
    let cols_per_row = img.width() as f32 / img.height() as f32 * 2.0;
    let mut rows = (h as f32 * rng.range(0.82, 1.0)).max(1.0) as u32;
    let mut cols = (rows as f32 * cols_per_row) as u32;
    if cols > w as u32 {
        cols = w as u32;
        rows = ((cols as f32 / cols_per_row) as u32).max(1);
    }
    let cols = cols.max(1);
    let small = image::imageops::resize(img, cols, rows, FilterType::Triangle);
    let small = image::imageops::unsharpen(&small, 1.0, 4);

    let pixels: Vec<Rgb> = small.pixels().map(|p| Rgb(p[0], p[1], p[2])).collect();
    let lums: Vec<f32> = pixels.iter().map(|&c| luminance(c)).collect();
    // Wyrównanie histogramu: jasność → percentyl. Każdy znak rampy dostaje
    // podobnie dużo komórek, więc twarz (same półtony) nie zlewa się w jedną plamę.
    let mut sorted = lums.clone();
    sorted.sort_by(f32::total_cmp);
    let dark_bg = luminance(pal.bg) < 0.5;
    let density: Vec<f32> = lums
        .iter()
        .map(|&l| {
            let v = sorted.partition_point(|&s| s < l) as f32 / sorted.len() as f32;
            // Gamma > 1 rozrzedza najciemniejsze tło za papieżem.
            if dark_bg { v } else { 1.0 - v }.powf(1.3)
        })
        .collect();

    let ramp: Vec<char> = RAMPS[rng.below(RAMPS.len())].chars().collect();
    let free_x = w as i32 - cols as i32;
    let x0 = free_x / 2 + (rng.range(-0.25, 0.25) * free_x as f32) as i32;
    let y0 = (h as i32 - rows as i32) / 2;
    let ink_tint = if dark_bg { Rgb(255, 255, 255) } else { Rgb(0, 0, 0) };

    let blank = Cell { ch: ' ', fg: pal.fg, bg: pal.bg };
    let mut cells = vec![blank; w as usize * h as usize];
    for y in 0..rows as i32 {
        for x in 0..cols as i32 {
            let (tx, ty) = (x0 + x, y0 + y);
            if tx < 0 || ty < 0 || tx >= w as i32 || ty >= h as i32 {
                continue;
            }
            let i = (y * cols as i32 + x) as usize;
            let idx = (density[i] * (ramp.len() - 1) as f32).round() as usize;
            cells[ty as usize * w as usize + tx as usize] = Cell {
                ch: ramp[idx],
                fg: pixels[i].lerp(ink_tint, 0.3),
                bg: pal.bg,
            };
        }
    }
    Grid { w, h, cells }
}
