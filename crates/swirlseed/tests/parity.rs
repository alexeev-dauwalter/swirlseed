//! Golden tests. `testdata/golden.txt` is shared with the npm package
//! (packages/swirlseed/test/parity.test.mjs): both implementations must produce exactly the same
//! SHA-256 for every case. Regenerate the file from the TypeScript side with `npm run golden`.

use swirlseed::dmath;
use swirlseed::{
    create_pattern, generate, palette_hex, DitherOptions, GrainOptions, HalftoneMode, HalftoneOptions, HalftoneShape,
    RenderOptions, RibsOptions,
};

#[path = "sha256.rs"]
mod sha256;

fn sha_hex(bytes: &[u8]) -> String {
    sha256::hex(bytes)
}

/// Keep this list identical to `CASES` in packages/swirlseed/test/parity.test.mjs.
fn cases() -> Vec<(&'static str, &'static str, u32, u32, RenderOptions)> {
    let none = RenderOptions::default;
    vec![
        ("base-hello", "Hello, world!", 128, 128, none()),
        ("base-nonsquare", "Hello, world", 160, 96, none()),
        (
            "no-shine-cyrillic",
            "Пётр",
            128,
            128,
            RenderOptions {
                shine: Some(0.0),
                ..none()
            },
        ),
        (
            "shine-strong",
            "alxv.life",
            128,
            128,
            RenderOptions {
                shine: Some(1.2),
                ..none()
            },
        ),
        (
            "ribs-explicit",
            "Autelon",
            144,
            144,
            RenderOptions {
                ribs: Some(RibsOptions {
                    count: Some(18.0),
                    strength: Some(0.7),
                    angle: Some(0.3),
                    shade: Some(0.5),
                }),
                ..none()
            },
        ),
        (
            "ribs-default",
            "glass",
            128,
            128,
            RenderOptions {
                ribs: Some(RibsOptions::default()),
                ..none()
            },
        ),
        (
            "halftone-default",
            "Granite",
            128,
            128,
            RenderOptions {
                halftone: Some(HalftoneOptions::default()),
                ..none()
            },
        ),
        (
            "halftone-square-mono-invert",
            "Kinsward",
            130,
            130,
            RenderOptions {
                halftone: Some(HalftoneOptions {
                    size: Some(9.0),
                    angle: Some(0.2),
                    shape: Some(HalftoneShape::Square),
                    mode: Some(HalftoneMode::Mono),
                    invert: Some(true),
                    mix: Some(0.8),
                }),
                ..none()
            },
        ),
        (
            "halftone-line",
            "42",
            140,
            140,
            RenderOptions {
                halftone: Some(HalftoneOptions {
                    shape: Some(HalftoneShape::Line),
                    mode: Some(HalftoneMode::Mono),
                    size: Some(8.0),
                    angle: Some(0.3),
                    ..Default::default()
                }),
                ..none()
            },
        ),
        (
            "dither-explicit",
            "seed",
            128,
            128,
            RenderOptions {
                dither: Some(DitherOptions {
                    levels: Some(4.0),
                    scale: Some(1.5),
                    amount: Some(1.0),
                }),
                ..none()
            },
        ),
        (
            "dither-default",
            "Hermes",
            128,
            128,
            RenderOptions {
                dither: Some(DitherOptions::default()),
                ..none()
            },
        ),
        (
            "grain",
            "Narra",
            128,
            128,
            RenderOptions {
                grain: Some(GrainOptions {
                    amount: Some(0.5),
                    size: Some(2.0),
                }),
                ..none()
            },
        ),
        (
            "all-effects",
            "neural networks",
            200,
            120,
            RenderOptions {
                ribs: Some(RibsOptions {
                    count: Some(20.0),
                    ..Default::default()
                }),
                halftone: Some(HalftoneOptions {
                    size: Some(7.0),
                    mix: Some(0.5),
                    ..Default::default()
                }),
                grain: Some(GrainOptions::default()),
                dither: Some(DitherOptions {
                    levels: Some(8.0),
                    ..Default::default()
                }),
                ..none()
            },
        ),
        ("empty-seed", "", 64, 64, none()),
        ("size-512", "Hello, world!", 512, 512, none()),
    ]
}

fn dmath_digest() -> String {
    let mut buf: Vec<u8> = Vec::with_capacity(2000 * 5 * 8);
    let mut put = |v: f64| buf.extend_from_slice(&v.to_be_bytes());
    for i in 0..2000i32 {
        let x = (i - 1000) as f64 * 0.0137;
        put(dmath::sin(x));
        put(dmath::cos(x));
        put(dmath::exp((i - 1000) as f64 * 0.04));
        put(dmath::ln(0.001 + i as f64 * 0.01));
        put(dmath::pow(i as f64 / 2000.0, 0.8 + (i % 7) as f64 * 0.1));
    }
    sha_hex(&buf)
}

fn moods_digest() -> String {
    let mut s = String::new();
    for i in 0..40 {
        s.push_str(&sha_hex(&generate(
            &format!("seed{i}"),
            48,
            48,
            &RenderOptions::default(),
        )));
    }
    sha_hex(s.as_bytes())
}

#[test]
fn sha256_known_vectors() {
    assert_eq!(
        sha_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn golden_hashes_match_typescript() {
    let golden = include_str!("../testdata/golden.txt");

    let mut actual = String::new();
    for (name, seed, w, h, opts) in cases() {
        actual.push_str(&format!("{name}\t{}\n", sha_hex(&generate(seed, w, h, &opts))));
    }
    actual.push_str(&format!("dmath\t{}\n", dmath_digest()));
    actual.push_str(&format!("moods\t{}\n", moods_digest()));

    // report every mismatching case, not just the first
    let want: Vec<&str> = golden.lines().collect();
    let got: Vec<&str> = actual.lines().collect();
    assert_eq!(want.len(), got.len(), "different number of golden cases");
    let bad: Vec<String> = want
        .iter()
        .zip(&got)
        .filter(|(a, b)| a != b)
        .map(|(a, b)| format!("  want {a}\n  got  {b}"))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust output differs from TypeScript:\n{}",
        bad.join("\n")
    );
}

#[test]
fn deterministic_math_is_accurate() {
    for i in 0..4000 {
        let x = (i - 2000) as f64 * 0.0173;
        assert!((dmath::sin(x) - x.sin()).abs() < 1e-12, "sin {x}");
        assert!((dmath::cos(x) - x.cos()).abs() < 1e-12, "cos {x}");
        let e = (i - 2000) as f64 * 0.01;
        assert!((dmath::exp(e) / e.exp() - 1.0).abs() < 1e-12, "exp {e}");
        let l = 1e-6 + i as f64 * 0.25;
        assert!((dmath::ln(l) - l.ln()).abs() < 1e-12, "ln {l}");
        let t = (i + 1) as f64 / 4001.0;
        assert!((dmath::pow(t, 1.37) / t.powf(1.37) - 1.0).abs() < 1e-11, "pow {t}");
    }
}

#[test]
fn same_seed_same_pixels_and_one_char_changes_everything() {
    let o = RenderOptions::default();
    let a = generate("Hello, world!", 96, 96, &o);
    assert_eq!(a, generate("Hello, world!", 96, 96, &o));
    for other in ["Hello, world", "hello, world!", "Hello, world?"] {
        let b = generate(other, 96, 96, &o);
        let diff: u64 = a
            .chunks(4)
            .zip(b.chunks(4))
            .map(|(p, q)| {
                (0..3)
                    .map(|c| (p[c] as i32 - q[c] as i32).unsigned_abs() as u64)
                    .sum::<u64>()
            })
            .sum();
        assert!(
            diff as f64 / (96.0 * 96.0 * 3.0) > 20.0,
            "\"{other}\" looks too similar"
        );
    }
}

#[cfg(feature = "nfc")]
#[test]
fn nfc_precomposed_and_combining_give_the_same_pattern() {
    let o = RenderOptions::default();
    assert_eq!(generate("caf\u{e9}", 64, 64, &o), generate("cafe\u{301}", 64, 64, &o));
}

#[test]
fn params_are_inspectable() {
    let p = create_pattern("Hello, world!");
    let hex = palette_hex(&p);
    assert!(hex.len() >= 4 && hex.iter().all(|h| h.len() == 7 && h.starts_with('#')));
}
