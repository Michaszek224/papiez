mod color;
mod grid;
mod morph;
mod pope;
mod rng;

use std::error::Error;
use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};

use grid::Grid;
use morph::{Canvas, Flight, Letters, play};
use rng::Rng;

const USAGE: &str = "\
papiez — litery zlatują się i układają w Jana Pawła II

UŻYCIE:
  papiez     dowolny klawisz = litery się rozlatują i wracasz do terminala";

const ARRIVE: Duration = Duration::from_millis(3800);
const LEAVE: Duration = Duration::from_millis(2000);
/// Co ile papież "oddycha" — kilka liter na chwilę zmienia się w inne.
const SHIMMER: Duration = Duration::from_millis(70);

fn main() {
    if std::env::args().skip(1).any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return;
    }
    std::panic::set_hook(Box::new(|info| {
        restore_terminal();
        eprintln!("{info}");
    }));
    if let Err(e) = run() {
        restore_terminal();
        eprintln!("papiez: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let (cols, rows) = terminal::size()?;
    terminal::enable_raw_mode()?;
    let probe = color::probe();
    let pal = &probe.palette;
    let mut rng = Rng::from_time();
    let mut pope = pope::ascii(cols, rows, pal, &mut rng);

    // Alternatywny ekran: po wszystkim terminal sam przywróci to, co było.
    let mut out = io::stdout().lock();
    execute!(out, EnterAlternateScreen, cursor::Hide)?;
    let mut canvas = Canvas::new(cols, rows);
    canvas.draw(&mut out, &Grid::blank(cols, rows, pal.fg, pal.bg))?;
    let arrive = Flight::arrive(&pope, probe.cursor, pal, &mut rng);
    play(&mut out, &mut canvas, &arrive, ARRIVE)?;

    let mut letters = Letters::new(&mut rng);
    let mut frame = pope.clone();
    let mut ink: Vec<usize> = ink_cells(&pope);
    loop {
        if event::poll(SHIMMER)? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => break,
                Event::Resize(w, h) => {
                    pope = pope::ascii(w, h, pal, &mut rng);
                    ink = ink_cells(&pope);
                    canvas = Canvas::new(w, h);
                    frame = pope.clone();
                }
                _ => {}
            }
        }
        frame.cells.copy_from_slice(&pope.cells);
        for _ in 0..(ink.len() / 150).max(1) {
            if let Some(&i) = ink.get(rng.below(ink.len())) {
                frame.cells[i].ch = letters.next();
            }
        }
        canvas.draw(&mut out, &frame)?;
    }

    let leave = Flight::leave(&pope, probe.cursor, pal, &mut rng);
    play(&mut out, &mut canvas, &leave, LEAVE)?;

    execute!(out, LeaveAlternateScreen, cursor::Show)?;
    out.flush()?;
    terminal::disable_raw_mode()?;
    Ok(())
}

fn ink_cells(g: &Grid) -> Vec<usize> {
    (0..g.cells.len()).filter(|&i| g.cells[i].ch != ' ').collect()
}

fn restore_terminal() {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
}
