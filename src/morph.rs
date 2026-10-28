use std::f32::consts::TAU;
use std::fmt::Write as _;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::color::{Palette, Rgb};
use crate::grid::{Cell, Grid};
use crate::rng::Rng;

const FPS: f32 = 60.0;
/// Z tych słów składają się latające litery.
const WORDS: &[&str] = &[
    "JAN PAWEŁ II",
    "BARKA",
    "KREMÓWKI",
    "WADOWICE",
    "2137",
    "TOTUS TUUS",
    "NIE LĘKAJCIE SIĘ",
    "SANTO SUBITO",
    "KAROL WOJTYŁA",
    "PAPAMOBILE",
];

/// Skąd litery przylatują.
#[derive(Clone, Copy, Debug)]
enum Spawn {
    /// Zza wszystkich krawędzi ekranu.
    Edges,
    /// Spadają z góry jak deszcz.
    Rain,
    /// Wybuchają z miejsca, gdzie wpisano `papiez`.
    Prompt,
    /// Pojawiają się w powietrzu w losowych miejscach.
    Materialize,
    /// Wpadają spiralą z dużego okręgu.
    Vortex,
}

/// W jakiej kolejności zapełnia się portret.
#[derive(Clone, Copy, Debug)]
enum Order {
    Random,
    TopDown,
    BottomUp,
    CenterOut,
    OutsideIn,
    Sweep,
}

#[derive(Clone, Copy)]
struct Particle {
    home: (f32, f32),
    ctrl: (f32, f32),
    dest: (f32, f32),
    ch_fly: char,
    ch_dest: char,
    fg_home: Rgb,
    fg_dest: Rgb,
    delay: f32,
    dur: f32,
}

/// Losowy, ale z góry ustalony plan lotu liter. Klatka zależy tylko od czasu,
/// więc rozlot to po prostu plan odtworzony od końca.
pub struct Flight {
    blank: Grid,
    target: Grid,
    particles: Vec<Particle>,
    leaving: bool,
}

impl Flight {
    /// Litery zlatują się w portret.
    pub fn arrive(target: &Grid, cursor: Option<(u16, u16)>, pal: &Palette, rng: &mut Rng) -> Flight {
        Self::plan(target, cursor, pal, rng, false)
    }

    /// Litery rozlatują się z portretu.
    pub fn leave(target: &Grid, cursor: Option<(u16, u16)>, pal: &Palette, rng: &mut Rng) -> Flight {
        Self::plan(target, cursor, pal, rng, true)
    }

    fn plan(
        target: &Grid,
        cursor: Option<(u16, u16)>,
        pal: &Palette,
        rng: &mut Rng,
        leaving: bool,
    ) -> Flight {
        let (w, h) = (target.w as f32, target.h as f32);
        let center = (w / 2.0, h / 2.0);
        let spawn = match rng.below(5) {
            0 => Spawn::Edges,
            1 => Spawn::Rain,
            2 => Spawn::Prompt,
            3 => Spawn::Materialize,
            _ => Spawn::Vortex,
        };
        let order = match rng.below(6) {
            0 => Order::Random,
            1 => Order::TopDown,
            2 => Order::BottomUp,
            3 => Order::CenterOut,
            4 => Order::OutsideIn,
            _ => Order::Sweep,
        };
        // Wir (wszystkie łuki w tę samą stronę) albo chaos (każdy w swoją).
        let swirl = match spawn {
            Spawn::Vortex => Some(if rng.f32() < 0.5 { 1.0 } else { -1.0 }),
            _ if rng.f32() < 0.4 => Some(if rng.f32() < 0.5 { 1.0 } else { -1.0 }),
            _ => None,
        };
        let curve = match spawn {
            Spawn::Rain => rng.range(0.0, 0.08),
            Spawn::Vortex => rng.range(0.35, 0.6),
            _ => rng.range(0.1, 0.6),
        };
        let spread = rng.range(0.3, 0.55);
        let prompt = cursor.map_or((0.0, h - 1.0), |(x, y)| (x as f32, y as f32));
        let max_r = (center.0 * center.0 + (center.1 * 2.0).powi(2)).sqrt();

        let order_key = |(x, y): (f32, f32), rng: &mut Rng| -> f32 {
            let r = ((x - center.0).powi(2) + ((y - center.1) * 2.0).powi(2)).sqrt() / max_r;
            match order {
                Order::Random => rng.f32(),
                Order::TopDown => y / h,
                Order::BottomUp => 1.0 - y / h,
                Order::CenterOut => r,
                Order::OutsideIn => 1.0 - r,
                Order::Sweep => x / w,
            }
        };

        let mut particles = Vec::new();
        for (i, cell) in target.cells.iter().enumerate() {
            if cell.ch == ' ' {
                continue;
            }
            let dest = ((i % target.w as usize) as f32, (i / target.w as usize) as f32);
            let home = match spawn {
                Spawn::Edges => match rng.below(4) {
                    0 => (rng.range(0.0, w), -2.0),
                    1 => (rng.range(0.0, w), h + 1.0),
                    2 => (-3.0, rng.range(0.0, h)),
                    _ => (w + 2.0, rng.range(0.0, h)),
                },
                Spawn::Rain => (dest.0 + rng.range(-6.0, 6.0), -rng.range(1.0, h * 0.8)),
                Spawn::Prompt => (prompt.0 + rng.range(-1.0, 6.0), prompt.1),
                Spawn::Materialize => (rng.range(0.0, w), rng.range(0.0, h)),
                Spawn::Vortex => {
                    let a = rng.f32() * TAU;
                    (center.0 + a.cos() * max_r, center.1 + a.sin() * max_r / 2.0)
                }
            };
            let (dx, dy) = (dest.0 - home.0, dest.1 - home.1);
            let sign = swirl.unwrap_or_else(|| if rng.f32() < 0.5 { 1.0 } else { -1.0 });
            let bend = sign * curve * rng.range(0.5, 1.5);
            // Prostopadle do toru; y liczony w "kwadratowych" jednostkach (komórka ~1:2).
            let ctrl = (home.0 + dx / 2.0 - dy * 2.0 * bend, home.1 + dy / 2.0 + dx / 2.0 * bend);
            let dur = rng.range(0.3, 0.5);
            let delay = (order_key(dest, rng) * spread + rng.range(0.0, 0.12)).min(1.0 - dur);
            particles.push(Particle {
                home,
                ctrl,
                dest,
                ch_fly: ' ',
                ch_dest: cell.ch,
                fg_home: if matches!(spawn, Spawn::Materialize) { pal.bg } else { pal.fg },
                fg_dest: cell.fg,
                delay,
                dur,
            });
        }
        // Litery po kolei ze strumienia słów, ale startują według `delay` —
        // sortujemy, żeby te same słowa leciały mniej więcej razem.
        let mut by_delay: Vec<usize> = (0..particles.len()).collect();
        by_delay.sort_by(|&a, &b| particles[a].delay.total_cmp(&particles[b].delay));
        let mut letters = Letters::new(rng);
        for i in by_delay {
            particles[i].ch_fly = letters.next();
        }

        // Rozciągamy harmonogram na cały czas animacji: pierwsza litera rusza
        // w klatce 0, ostatnia ląduje w ostatniej — bez martwych chwil na
        // początku i na końcu (ważne zwłaszcza przy odtwarzaniu od tyłu).
        let first = particles.iter().map(|p| p.delay).fold(f32::INFINITY, f32::min);
        let last = particles.iter().map(|p| p.delay + p.dur).fold(0.0, f32::max);
        if last > first {
            for p in &mut particles {
                p.delay = (p.delay - first) / (last - first);
                p.dur /= last - first;
            }
        }

        Flight {
            blank: Grid::blank(target.w, target.h, pal.fg, pal.bg),
            target: target.clone(),
            particles,
            leaving,
        }
    }

    /// Klatka w chwili `u` ∈ [0, 1] (0 = pusto, 1 = gotowy portret).
    fn frame(&self, u: f32, out: &mut Grid) {
        if u >= 1.0 {
            out.cells.copy_from_slice(&self.target.cells);
            return;
        }
        out.cells.copy_from_slice(&self.blank.cells);
        let (w, h) = (out.w as i32, out.h as i32);
        for p in &self.particles {
            let t = ((u - p.delay) / p.dur).clamp(0.0, 1.0);
            if t == 0.0 {
                continue;
            }
            // Przy nalocie litery startują pełnym pędem i zwalniają przy lądowaniu;
            // przy rozlocie (czas płynie wstecz) odwrotnie — więc też od razu pędzą.
            let e = if self.leaving { ease_in(t) } else { ease_out(t) };
            let bez = |a: f32, c: f32, b: f32| {
                (1.0 - e) * (1.0 - e) * a + 2.0 * (1.0 - e) * e * c + e * e * b
            };
            let (x, y) = (
                bez(p.home.0, p.ctrl.0, p.dest.0).round() as i32,
                bez(p.home.1, p.ctrl.1, p.dest.1).round() as i32,
            );
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let cell = &mut out.cells[(y * w + x) as usize];
            cell.ch = if t > 0.92 { p.ch_dest } else { p.ch_fly };
            cell.fg = p.fg_home.lerp(p.fg_dest, e);
        }
    }
}

/// Nieskończony strumień liter ze słów w losowej kolejności (bez spacji).
pub struct Letters {
    words: Vec<&'static str>,
    word: usize,
    chars: std::str::Chars<'static>,
}

impl Letters {
    pub fn new(rng: &mut Rng) -> Letters {
        let mut words = WORDS.to_vec();
        rng.shuffle(&mut words);
        let chars = words[0].chars();
        Letters { words, word: 0, chars }
    }

    pub fn next(&mut self) -> char {
        loop {
            match self.chars.next() {
                Some(' ') => {}
                Some(c) => return c,
                None => {
                    self.word = (self.word + 1) % self.words.len();
                    self.chars = self.words[self.word].chars();
                }
            }
        }
    }
}

/// Odtwarza lot w czasie `dur`; rozlot leci od końca.
pub fn play(out: &mut impl Write, canvas: &mut Canvas, flight: &Flight, dur: Duration) -> io::Result<()> {
    let mut frame = flight.blank.clone();
    let start = Instant::now();
    let tick = Duration::from_secs_f32(1.0 / FPS);
    loop {
        let t = (start.elapsed().as_secs_f32() / dur.as_secs_f32()).min(1.0);
        flight.frame(if flight.leaving { 1.0 - t } else { t }, &mut frame);
        canvas.draw(out, &frame)?;
        if t >= 1.0 {
            return Ok(());
        }
        let n = (start.elapsed().as_secs_f32() * FPS).floor() + 1.0;
        std::thread::sleep((start + tick.mul_f32(n)).saturating_duration_since(Instant::now()));
    }
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_in(t: f32) -> f32 {
    t * t * t
}

/// Śledzi, co już jest na ekranie, i wysyła tylko zmienione komórki.
pub struct Canvas {
    w: u16,
    h: u16,
    shown: Vec<Option<Cell>>,
}

impl Canvas {
    pub fn new(w: u16, h: u16) -> Canvas {
        Canvas { w, h, shown: vec![None; w as usize * h as usize] }
    }

    pub fn draw(&mut self, out: &mut impl Write, frame: &Grid) -> io::Result<()> {
        debug_assert!(frame.w == self.w && frame.h == self.h);
        // Synchronized output (CSI ?2026) — terminal pokaże klatkę w całości, bez rwania.
        let mut s = String::from("\x1b[?2026h");
        let mut cursor = None;
        let (mut fg, mut bg): (Option<Rgb>, Option<Rgb>) = (None, None);
        for y in 0..self.h {
            for x in 0..self.w {
                let i = y as usize * self.w as usize + x as usize;
                let c = frame.cells[i];
                if self.shown[i] == Some(c) {
                    continue;
                }
                if cursor != Some((x, y)) {
                    let _ = write!(s, "\x1b[{};{}H", y + 1, x + 1);
                }
                if fg != Some(c.fg) {
                    let _ = write!(s, "\x1b[38;2;{};{};{}m", c.fg.0, c.fg.1, c.fg.2);
                    fg = Some(c.fg);
                }
                if bg != Some(c.bg) {
                    let _ = write!(s, "\x1b[48;2;{};{};{}m", c.bg.0, c.bg.1, c.bg.2);
                    bg = Some(c.bg);
                }
                s.push(c.ch);
                cursor = Some((x + 1, y));
                self.shown[i] = Some(c);
            }
        }
        s.push_str("\x1b[0m\x1b[?2026l");
        out.write_all(s.as_bytes())?;
        out.flush()
    }
}
