use std::io::Cursor;
use std::sync::OnceLock;

use image::{ImageFormat, ImageReader, RgbImage, imageops::FilterType};

use crate::color::{Palette, Rgb};
use crate::grid::{Cell, Grid};
use crate::rng::Rng;

/// Zdjęcie i prostokąt z papieżem (ułamki szerokości/wysokości: x0, y0, x1, y1).
/// Kadr zawsze obejmuje ten prostokąt, a resztę terminala wypełnia tłem —
/// dlatego zdjęcia mają gładkie tło: w ASCII tłum za papieżem robi się szumem.
/// Źródła i licencje — w README.
struct Photo {
    jpg: &'static [u8],
    subject: [f32; 4],
}

const PHOTOS: &[Photo] = &[
    // Ręce rozłożone, niebieskie tło — obszar ~1.6:1, na szerokie terminale.
    Photo { jpg: include_bytes!("../assets/jp2-wave.jpg"), subject: [0.02, 0.07, 1.0, 0.55] },
    // Uśmiech, głowa i ramiona — ~1.2:1.
    Photo { jpg: include_bytes!("../assets/jp2-smile.jpg"), subject: [0.10, 0.05, 0.95, 0.62] },
    // Portret z 1984, do pasa — ~0.9:1.
    Photo { jpg: include_bytes!("../assets/jp2-tall.jpg"), subject: [0.05, 0.04, 0.95, 0.70] },
    // Cała postać, 1987 — ~0.5:1, na wąskie i wysokie terminale.
    Photo { jpg: include_bytes!("../assets/jp2-full.jpg"), subject: [0.10, 0.05, 0.92, 1.0] },
];

/// Ile zapasu dookoła papieża zostawić.
const MARGIN: f32 = 1.06;
/// Zdjęcia, które wypełniają terminal prawie tak dobrze jak najlepsze, są równie
/// dobre — losujemy spośród nich, żeby nie było za każdym razem tego samego.
const TIE: f32 = 0.85;
/// Rampy od "pustego" do "pełnego" znaku; przy każdej przemianie losujemy jedną.
const RAMPS: &[&str] = &[" .:-=+*#%@", " .,:;irsXA253hMHGS#9B&@", " .:;+=xX$&"];
/// Ile gęstości zostaje tłu poza papieżem.
const BACKGROUND: f32 = 0.15;

fn decoded(i: usize) -> &'static RgbImage {
    static DECODED: [OnceLock<RgbImage>; PHOTOS.len()] = [const { OnceLock::new() }; PHOTOS.len()];
    DECODED[i].get_or_init(|| {
        image::load_from_memory_with_format(PHOTOS[i].jpg, ImageFormat::Jpeg)
            .expect("wbudowane zdjęcie jest poprawnym JPEG-iem")
            .to_rgb8()
    })
}

/// Rozmiar z samego nagłówka JPEG — nie trzeba dekodować zdjęć, których nie użyjemy.
fn dimensions(i: usize) -> (f32, f32) {
    let (w, h) = ImageReader::with_format(Cursor::new(PHOTOS[i].jpg), ImageFormat::Jpeg)
        .into_dimensions()
        .expect("wbudowane zdjęcie ma poprawny nagłówek");
    (w as f32, h as f32)
}

/// Prostokąt z papieżem w pikselach zdjęcia: (x, y, szer., wys.).
fn subject_px(i: usize) -> [f32; 4] {
    let (iw, ih) = dimensions(i);
    let [x0, y0, x1, y1] = PHOTOS[i].subject;
    [x0 * iw, y0 * ih, (x1 - x0) * iw, (y1 - y0) * ih]
}

/// Najmniejszy kadr o proporcjach `aspect`, w którym mieści się papież (z zapasem),
/// i jego ocena: jaką część terminala zajmuje papież (1.0 = idealnie).
/// Kadr może wystawać poza zdjęcie — tam będzie puste tło.
fn framing((iw, ih): (f32, f32), [sx, sy, sw, sh]: [f32; 4], aspect: f32) -> ([f32; 4], f32) {
    let (cw, ch) = if sw / sh < aspect { (sh * aspect, sh) } else { (sw, sw / aspect) };
    let (cw, ch) = (cw * MARGIN, ch * MARGIN);
    // Papież na środku; jeśli kadr mieści się w zdjęciu, nie wychodzimy poza nie.
    let place = |s: f32, size: f32, len: f32, img: f32| {
        let centered = s + size / 2.0 - len / 2.0;
        if len <= img { centered.clamp(0.0, img - len) } else { centered }
    };
    let crop = [place(sx, sw, cw, iw), place(sy, sh, ch, ih), cw, ch];
    (crop, (sw * sh) / (cw * ch))
}

/// Wybiera zdjęcie najlepiej pasujące do proporcji terminala; przy remisie losuje.
fn choose(aspect: f32, rng: &mut Rng) -> (usize, [f32; 4]) {
    let options: Vec<(usize, [f32; 4], f32)> = (0..PHOTOS.len())
        .map(|i| {
            let (crop, score) = framing(dimensions(i), subject_px(i), aspect);
            (i, crop, score)
        })
        .collect();
    let best = options.iter().map(|o| o.2).fold(0.0, f32::max);
    let good: Vec<_> = options.into_iter().filter(|o| o.2 >= best * TIE).collect();
    let (i, crop, _) = good[rng.below(good.len())];
    (i, crop)
}

pub fn luminance(c: Rgb) -> f32 {
    (0.2126 * c.0 as f32 + 0.7152 * c.1 as f32 + 0.0722 * c.2 as f32) / 255.0
}

/// Papież z ASCII na cały terminal. Gęstość znaku odpowiada jasności zdjęcia —
/// na ciemnym tle jasna sutanna to `@`, na jasnym odwrotnie.
pub fn ascii(w: u16, h: u16, pal: &Palette, rng: &mut Rng) -> Grid {
    // Komórka terminala jest ~2× wyższa niż szersza.
    let aspect = w as f32 / (h as f32 * 2.0);
    let (i, crop) = choose(aspect, rng);
    let ramp = rng.below(RAMPS.len());
    ascii_with(i, crop, w, h, pal, ramp)
}

/// Renderuje kadr `crop` zdjęcia `i` rampą `ramp` na siatkę `w`×`h`.
fn ascii_with(i: usize, [cx, cy, cw, ch]: [f32; 4], w: u16, h: u16, pal: &Palette, ramp: usize) -> Grid {
    let (iw, ih) = dimensions(i);
    let to_x = |px: f32| (px - cx) / cw * w as f32;
    let to_y = |py: f32| (py - cy) / ch * h as f32;

    // Część kadru, która leży na zdjęciu, skalujemy na jej miejsce w terminalu;
    // reszta zostaje pusta.
    let (ix0, iy0, ix1, iy1) = (cx.max(0.0), cy.max(0.0), (cx + cw).min(iw), (cy + ch).min(ih));
    let (ox0, oy0) = (to_x(ix0).round().max(0.0) as u32, to_y(iy0).round().max(0.0) as u32);
    let (ox1, oy1) = (to_x(ix1).round().min(w as f32) as u32, to_y(iy1).round().min(h as f32) as u32);
    let region = image::imageops::crop_imm(
        decoded(i),
        ix0 as u32,
        iy0 as u32,
        (ix1 - ix0).max(1.0) as u32,
        (iy1 - iy0).max(1.0) as u32,
    );
    let small = image::imageops::resize(
        &*region,
        ox1.saturating_sub(ox0).max(1),
        oy1.saturating_sub(oy0).max(1),
        FilterType::Triangle,
    );
    let small = image::imageops::unsharpen(&small, 1.0, 4);
    let mut pixels: Vec<Option<Rgb>> = vec![None; w as usize * h as usize];
    for (x, y, p) in small.enumerate_pixels() {
        let (tx, ty) = (ox0 + x, oy0 + y);
        if tx < w as u32 && ty < h as u32 {
            pixels[(ty * w as u32 + tx) as usize] = Some(Rgb(p[0], p[1], p[2]));
        }
    }

    // Miękki zaokrąglony prostokąt wokół papieża: 1 w środku, 0 daleko.
    // Wykładnik 4 zamiast 2 (elipsy), żeby nie obcinać rąk w rogach.
    let [sx, sy, sw, sh] = subject_px(i);
    let (ex, ey) = (to_x(sx + sw / 2.0), to_y(sy + sh / 2.0));
    let (rx, ry) = (sw / cw * w as f32 / 2.0 * 1.1, sh / ch * h as f32 / 2.0 * 1.1);
    let mask: Vec<f32> = (0..pixels.len())
        .map(|k| {
            let (x, y) = ((k % w as usize) as f32 + 0.5, (k / w as usize) as f32 + 0.5);
            let d = (((x - ex) / rx).powi(4) + ((y - ey) / ry).powi(4)).powf(0.25);
            1.0 - smoothstep(1.0, 1.4, d)
        })
        .collect();

    // Wyrównanie histogramu: jasność → percentyl. Każdy znak rampy dostaje
    // podobnie dużo komórek, więc twarz (same półtony) nie zlewa się w jedną plamę.
    // Liczone z całego zdjęcia: gdyby tylko z papieża, biała sutanna zajęłaby
    // górę skali i twarz wpadłaby w jeden znak.
    let mut sorted: Vec<f32> = pixels.iter().flatten().map(|&p| luminance(p)).collect();
    sorted.sort_by(f32::total_cmp);
    let dark_bg = luminance(pal.bg) < 0.5;
    let ramp: Vec<char> = RAMPS[ramp].chars().collect();
    let ink_tint = if dark_bg { Rgb(255, 255, 255) } else { Rgb(0, 0, 0) };

    let cells = (0..pixels.len())
        .map(|k| {
            let Some(px) = pixels[k] else {
                return Cell { ch: ' ', fg: pal.fg, bg: pal.bg };
            };
            let v = sorted.partition_point(|&s| s < luminance(px)) as f32 / sorted.len().max(1) as f32;
            // Gamma > 1 rozrzedza najciemniejsze tło za papieżem.
            let tone = if dark_bg { v } else { 1.0 - v }.powf(1.3);
            let density = tone * (BACKGROUND + (1.0 - BACKGROUND) * mask[k]);
            let ch = ramp[(density * (ramp.len() - 1) as f32).round() as usize];
            Cell { ch, fg: px.lerp(ink_tint, 0.3), bg: pal.bg }
        })
        .collect();
    Grid { w, h, cells }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn best(cols: u16, rows: u16) -> usize {
        let aspect = cols as f32 / (rows as f32 * 2.0);
        let scores: Vec<f32> =
            (0..PHOTOS.len()).map(|i| framing(dimensions(i), subject_px(i), aspect).1).collect();
        (0..scores.len()).max_by(|&a, &b| scores[a].total_cmp(&scores[b])).unwrap()
    }

    #[test]
    fn picks_photo_by_terminal_shape() {
        assert_eq!(best(220, 50), 0, "szeroki terminal → rozłożone ręce");
        assert_eq!(best(50, 60), 3, "wąski i wysoki → cała postać");
    }

    #[test]
    fn crop_has_terminal_shape_and_whole_subject() {
        for i in 0..PHOTOS.len() {
            let [sx, sy, sw, sh] = subject_px(i);
            for aspect in [0.3, 0.6, 1.0, 1.6, 2.4, 4.0] {
                let ([x, y, w, h], score) = framing(dimensions(i), subject_px(i), aspect);
                assert!((w / h - aspect).abs() < 0.01, "kadr ma proporcje terminala");
                assert!(x <= sx && y <= sy && x + w >= sx + sw && y + h >= sy + sh, "papież w kadrze");
                assert!(score > 0.0 && score <= 1.0);
            }
        }
    }

    #[test]
    fn renders_any_terminal_size() {
        let pal = Palette { fg: Rgb(200, 200, 200), bg: Rgb(0, 0, 0) };
        let mut rng = Rng::from_time();
        for (w, h) in [(1, 1), (2, 40), (300, 3), (80, 24), (250, 70), (40, 80)] {
            for _ in 0..4 {
                let g = ascii(w, h, &pal, &mut rng);
                assert_eq!(g.cells.len(), w as usize * h as usize);
            }
        }
    }
}
