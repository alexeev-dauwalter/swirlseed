//! Everything the seed decides. Mirrors `params.ts`; the random numbers are consumed in exactly
//! the same order, so the same seed gives the same parameters in both languages.

use crate::color::{lab_to_srgb255, oklch_fit, Lab};
use crate::dmath::PI;
use crate::rng::{make_rng, Rng};

#[derive(Clone, Debug)]
pub struct WarpLayer {
    pub freq: f64,
    pub amp: f64,
    pub ox: f64,
    pub oy: f64,
    pub ox2: f64,
    pub oy2: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Stop {
    pub pos: f64,
    pub lab: Lab,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Classic,
    Night,
    Pastel,
    Mono,
}

impl Mood {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mood::Classic => "classic",
            Mood::Night => "night",
            Mood::Pastel => "pastel",
            Mood::Mono => "mono",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// One pass through the palette.
    Ramp,
    /// Several back-and-forth waves through the palette.
    Bands,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Ramp => "ramp",
            Mode::Bands => "bands",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Swirl {
    pub cx: f64,
    pub cy: f64,
    pub strength: f64,
    pub radius: f64,
}

/// Everything the seed decides. Inspectable; feed it to `render_pattern`.
#[derive(Clone, Debug)]
pub struct Params {
    pub seed: String,
    /// Noise permutation table (512 bytes, depends only on the seed).
    pub perm: Vec<u8>,
    /// Flow direction and how much the noise is stretched along it (long streaks).
    pub angle: f64,
    pub stretch: f64,
    pub base_freq: f64,
    pub swirl: Swirl,
    pub layers: Vec<WarpLayer>,
    pub mode: Mode,
    pub repeats: f64,
    pub phase: f64,
    pub fbm_weight: f64,
    pub gamma: f64,
    /// Default light/gloss strength (override with `RenderOptions::shine`).
    pub shine: f64,
    pub mood: Mood,
    pub stops: Vec<Stop>,
}

/// Hue pairs (base, accent) in degrees: teal + red-orange, blue + red, and so on.
const PAIRS: [(f64, f64); 9] = [
    (215.0, 32.0),
    (255.0, 28.0),
    (235.0, 55.0),
    (305.0, 45.0),
    (180.0, 20.0),
    (275.0, 18.0),
    (20.0, 215.0),
    (160.0, 25.0),
    (330.0, 200.0),
];

fn make_palette(rnd: &mut Rng) -> (Vec<Stop>, Mood) {
    let (base_a, base_b) = PAIRS[(rnd.next_f64() * PAIRS.len() as f64).floor() as usize];
    let hue_a = base_a + rnd.range(-14.0, 14.0);
    let hue_b = base_b + rnd.range(-14.0, 14.0);
    let hue_c = hue_b + rnd.range(14.0, 42.0);

    let r = rnd.next_f64();
    let mood = if r < 0.5 {
        Mood::Classic
    } else if r < 0.7 {
        Mood::Night
    } else if r < 0.86 {
        Mood::Pastel
    } else {
        Mood::Mono
    };

    let roles: Vec<Lab> = match mood {
        Mood::Classic => {
            let dark = oklch_fit(rnd.range(0.13, 0.24), rnd.range(0.02, 0.07), hue_a);
            let deep = oklch_fit(
                rnd.range(0.3, 0.46),
                rnd.range(0.07, 0.13),
                hue_a + rnd.range(-15.0, 15.0),
            );
            let accent = oklch_fit(rnd.range(0.54, 0.64), rnd.range(0.15, 0.21), hue_b);
            let warm = oklch_fit(rnd.range(0.62, 0.72), rnd.range(0.1, 0.16), hue_c);
            let light = oklch_fit(
                rnd.range(0.9, 0.96),
                rnd.range(0.005, 0.03),
                hue_a + rnd.range(-30.0, 30.0),
            );
            let q = rnd.next_f64();
            if q < 0.65 {
                vec![dark, deep, accent, warm, light]
            } else if q < 0.85 {
                vec![dark, deep, accent, warm]
            } else {
                vec![deep, accent, warm, light]
            }
        }
        Mood::Night => {
            // near-black background and a "glowing" accent
            let s0 = oklch_fit(rnd.range(0.07, 0.13), rnd.range(0.01, 0.04), hue_a);
            let s1 = oklch_fit(rnd.range(0.2, 0.3), rnd.range(0.06, 0.11), hue_a);
            let s2 = oklch_fit(
                rnd.range(0.42, 0.52),
                rnd.range(0.14, 0.2),
                hue_a + rnd.range(10.0, 30.0),
            );
            let s3 = oklch_fit(rnd.range(0.62, 0.72), rnd.range(0.17, 0.23), hue_b);
            let s4 = oklch_fit(
                rnd.range(0.88, 0.95),
                rnd.range(0.03, 0.08),
                hue_b + rnd.range(10.0, 30.0),
            );
            vec![s0, s1, s2, s3, s4]
        }
        Mood::Pastel => {
            let s0 = oklch_fit(rnd.range(0.58, 0.68), rnd.range(0.1, 0.14), hue_a);
            let s1 = oklch_fit(rnd.range(0.74, 0.82), rnd.range(0.06, 0.1), hue_b);
            let s2 = oklch_fit(
                rnd.range(0.93, 0.97),
                rnd.range(0.01, 0.03),
                hue_a + rnd.range(-20.0, 20.0),
            );
            let s3 = oklch_fit(rnd.range(0.8, 0.88), rnd.range(0.05, 0.09), hue_c);
            let s4 = oklch_fit(
                rnd.range(0.7, 0.78),
                rnd.range(0.07, 0.11),
                hue_a + rnd.range(30.0, 60.0),
            );
            vec![s0, s1, s2, s3, s4]
        }
        Mood::Mono => {
            // one hue from dark to light plus a tiny accent of another hue
            let h = hue_a;
            let peak = rnd.range(0.1, 0.17);
            let s0 = oklch_fit(rnd.range(0.12, 0.2), peak * 0.35, h);
            let s1 = oklch_fit(rnd.range(0.34, 0.42), peak * 0.75, h);
            let s2 = oklch_fit(rnd.range(0.54, 0.62), peak, h + rnd.range(-10.0, 10.0));
            let s3 = oklch_fit(rnd.range(0.68, 0.76), peak * 0.9, hue_b);
            let s4 = oklch_fit(rnd.range(0.9, 0.96), peak * 0.2, h);
            vec![s0, s1, s2, s3, s4]
        }
    };

    let n = roles.len();
    let mut stops: Vec<Stop> = Vec::with_capacity(n);
    for (i, c) in roles.iter().enumerate() {
        let jitter = if i == 0 || i == n - 1 {
            0.0
        } else {
            rnd.range(-0.07, 0.07)
        };
        let pos = (i as f64 / (n - 1) as f64 + jitter).max(0.0).min(1.0);
        stops.push(Stop { pos, lab: *c });
    }
    if rnd.next_f64() < 0.5 {
        stops.reverse();
        for s in stops.iter_mut() {
            s.pos = 1.0 - s.pos;
        }
    }
    (stops, mood)
}

/// Deterministically derive all pattern parameters from a seed string.
pub fn create_pattern(seed: &str) -> Params {
    // Independent streams: changing the palette code never moves the geometry, and vice versa.
    let mut noise_rng = make_rng(seed, "noise");
    let mut geo = make_rng(seed, "geo");
    let mut pal = make_rng(seed, "pal");

    let mut base = [0u8; 256];
    for (i, b) in base.iter_mut().enumerate() {
        *b = i as u8;
    }
    for i in (1..=255usize).rev() {
        let j = (noise_rng.next_f64() * (i + 1) as f64).floor() as usize;
        base.swap(i, j);
    }
    let mut perm = vec![0u8; 512];
    for (i, p) in perm.iter_mut().enumerate() {
        *p = base[i & 255];
    }

    let n_layers = 2 + (geo.next_f64() * 2.0).floor() as usize; // 2-3
    let mut layers = Vec::with_capacity(n_layers);
    for i in 0..n_layers {
        let fi = i as f64;
        layers.push(WarpLayer {
            freq: geo.range(0.45, 0.85) * (1.0 + fi * 0.4),
            amp: geo.range(0.75, 1.4) / (1.0 + fi * 0.4),
            ox: geo.range(-50.0, 50.0),
            oy: geo.range(-50.0, 50.0),
            ox2: geo.range(-50.0, 50.0),
            oy2: geo.range(-50.0, 50.0),
        });
    }

    let sign = if geo.next_f64() < 0.5 { -1.0 } else { 1.0 };
    let mode = if geo.next_f64() < 0.55 { Mode::Bands } else { Mode::Ramp };
    let (stops, mood) = make_palette(&mut pal);

    Params {
        seed: seed.to_owned(),
        perm,
        angle: geo.range(0.0, PI),
        stretch: geo.range(2.6, 6.0),
        base_freq: geo.range(1.3, 2.1),
        swirl: Swirl {
            cx: geo.range(-0.35, 0.35),
            cy: geo.range(-0.3, 0.3),
            strength: sign * geo.range(0.5, 2.0),
            radius: geo.range(0.3, 0.6),
        },
        layers,
        mode,
        repeats: geo.range(0.8, 2.2),
        phase: geo.next_f64(),
        fbm_weight: geo.range(0.15, 0.55),
        gamma: geo.range(0.8, 1.25),
        shine: geo.range(0.25, 0.6),
        mood,
        stops,
    }
}

/// The palette as `#rrggbb` strings, ordered by position along the ramp.
pub fn palette_hex(p: &Params) -> Vec<String> {
    let mut stops = p.stops.clone();
    stops.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap());
    stops
        .iter()
        .map(|s| {
            let rgb = lab_to_srgb255(&s.lab);
            let h = |v: f64| (v + 0.5).floor().max(0.0).min(255.0) as u8;
            format!("#{:02x}{:02x}{:02x}", h(rgb[0]), h(rgb[1]), h(rgb[2]))
        })
        .collect()
}
