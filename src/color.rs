use std::io::{Read, Write};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn lerp(self, to: Rgb, t: f32) -> Rgb {
        let f = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Rgb(f(self.0, to.0), f(self.1, to.1), f(self.2, to.2))
    }
}

/// Domyślne kolory terminala — żeby tło animacji zgadzało się z prawdziwym tłem.
#[derive(Clone, Debug)]
pub struct Palette {
    pub fg: Rgb,
    pub bg: Rgb,
}

/// To, czego dowiedzieliśmy się od terminala przed startem.
pub struct Probe {
    pub palette: Palette,
    /// (kolumna, wiersz) kursora, czyli mniej więcej tam, gdzie wpisano `papiez`.
    pub cursor: Option<(u16, u16)>,
}

/// Pyta terminal o kolory (OSC 10/11) i pozycję kursora (CSI 6n). Na końcu wysyła
/// DA1 (`CSI c`), na które odpowiada każdy terminal, więc wiemy kiedy przestać czekać.
/// Wymaga trybu raw.
pub fn probe() -> Probe {
    let mut probe = Probe {
        palette: Palette { fg: Rgb(229, 229, 229), bg: Rgb(0, 0, 0) },
        cursor: None,
    };
    let mut out = std::io::stdout();
    let query = "\x1b]10;?\x1b\\\x1b]11;?\x1b\\\x1b[6n\x1b[c";
    if out.write_all(query.as_bytes()).and_then(|_| out.flush()).is_err() {
        return probe;
    }

    let reply = read_until_da1(Duration::from_millis(500));
    for seg in reply.split("\x1b]").skip(1) {
        let seg = seg.split(['\x07', '\x1b']).next().unwrap_or("");
        match seg.split_once(';') {
            Some(("10", c)) => probe.palette.fg = parse_rgb(c).unwrap_or(probe.palette.fg),
            Some(("11", c)) => probe.palette.bg = parse_rgb(c).unwrap_or(probe.palette.bg),
            _ => {}
        }
    }
    probe.cursor = reply.split("\x1b[").find_map(|seg| {
        let (row, rest) = seg.split_once(';')?;
        let col = rest.strip_suffix('R')?;
        Some((col.parse::<u16>().ok()?.saturating_sub(1), row.parse::<u16>().ok()?.saturating_sub(1)))
    });
    probe
}

fn read_until_da1(timeout: Duration) -> String {
    let start = Instant::now();
    let mut buf = Vec::new();
    let mut stdin = std::io::stdin();
    while let Some(left) = timeout.checked_sub(start.elapsed()) {
        let mut pfd = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
        // SAFETY: jeden poprawny pollfd, fd 0 należy do procesu.
        let ready = unsafe { libc::poll(&mut pfd, 1, left.as_millis() as i32) };
        if ready <= 0 {
            break;
        }
        let mut chunk = [0u8; 1024];
        match stdin.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
        let s = String::from_utf8_lossy(&buf);
        if let Some(da) = s.rfind("\x1b[?") {
            if s[da..].contains('c') {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// `rgb:RRRR/GGGG/BBBB`; każda składowa może mieć od 1 do 4 cyfr hex.
fn parse_rgb(s: &str) -> Option<Rgb> {
    let mut it = s.strip_prefix("rgb:")?.split('/').map(|h| {
        let v = u32::from_str_radix(h, 16).ok()?;
        let max = (1u32 << (4 * h.len().clamp(1, 4))) - 1;
        Some((v * 255 / max) as u8)
    });
    Some(Rgb(it.next()??, it.next()??, it.next()??))
}
