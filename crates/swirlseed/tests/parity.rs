//! Golden tests. `testdata/golden.txt` is shared with the npm package
//! (packages/swirlseed/test/parity.test.mjs): both implementations must produce exactly the same
//! SHA-256 for every case. Regenerate the file from the TypeScript side with `npm run golden`.

use swirlseed::dmath;
use swirlseed::{
    create_pattern, generate, palette_hex, DitherOptions, GrainOptions, HalftoneMode, HalftoneOptions, HalftoneShape,
    RenderOptions, RibsOptions,
};

mod sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98,
        0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
        0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8,
        0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
        0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
        0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    pub fn hex(data: &[u8]) -> String {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
        ];
        let mut msg = data.to_vec();
        let bit_len = (data.len() as u64) * 8;
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());
        for chunk in msg.chunks(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
            }
            let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ (!e & g);
                let t1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
                *x = x.wrapping_add(y);
            }
        }
        h.iter().map(|v| format!("{:08x}", v)).collect()
    }
}

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
