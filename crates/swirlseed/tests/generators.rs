//! Golden tests for the generators. `testdata/generators.txt` holds the cases and their SHA-256 and is
//! shared with the npm package (packages/swirlseed/test/generators.test.mjs). Regenerate it from the
//! TypeScript side with `npm run golden`.

use swirlseed::dmath;
use swirlseed::gen::{get_generator, render_generator, resolve_params, Kind, Value, GENERATORS};

#[path = "sha256.rs"]
mod sha256;

const FIXTURE: &str = include_str!("../testdata/generators.txt");

fn parse_params(s: &str) -> Vec<(String, Value)> {
    if s == "-" {
        return Vec::new();
    }
    s.split(',')
        .map(|kv| {
            let (k, v) = kv.split_once('=').expect("key=value");
            let val = match v.parse::<f64>() {
                Ok(n) => Value::Num(n),
                Err(_) => Value::Str(v.to_string()),
            };
            (k.to_string(), val)
        })
        .collect()
}

fn dmath_digest() -> String {
    let mut buf = Vec::with_capacity(2000 * 5 * 8);
    for i in 0..2000 {
        let x = (i as f64 - 1000.0) * 0.0213;
        let u = (i as f64 - 1000.0) / 1000.0;
        for v in [
            dmath::atan(x),
            dmath::atan2(x, ((i % 13) - 6) as f64),
            dmath::asin(u),
            dmath::acos(u),
            dmath::cbrt(x * 7.3),
        ] {
            buf.extend_from_slice(&v.to_be_bytes());
        }
    }
    sha256::hex(&buf)
}

#[test]
fn generator_hashes_match_typescript() {
    let mut failed = Vec::new();
    let mut n = 0;
    for line in FIXTURE.lines().filter(|l| !l.is_empty()) {
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 7, "bad fixture line: {line}");
        let (name, id, seed, w, h, params, want) = (f[0], f[1], f[2], f[3], f[4], f[5], f[6]);
        let got = if name == "dmath-inverse" {
            dmath_digest()
        } else {
            let px = render_generator(id, seed, w.parse().unwrap(), h.parse().unwrap(), &parse_params(params)).unwrap();
            sha256::hex(&px)
        };
        n += 1;
        if got != want {
            failed.push(format!("{name}: got {got}, want {want}"));
        }
    }
    assert!(n > 40, "fixture looks empty");
    assert!(failed.is_empty(), "mismatches:\n{}", failed.join("\n"));
}

#[test]
fn every_generator_is_covered_and_has_valid_defaults() {
    for g in GENERATORS {
        assert!(
            FIXTURE.lines().any(|l| l.split('\t').nth(1) == Some(g.id)),
            "no golden case for {}",
            g.id
        );
        let px = render_generator(g.id, "x", 24, 16, &[]).unwrap();
        assert_eq!(px.len(), 24 * 16 * 4);
        for p in g.params {
            match p.kind {
                Kind::Number { min, max, default, .. } => {
                    assert!(default >= min && default <= max, "{}.{}", g.id, p.key)
                }
                Kind::Choice { default, options } => {
                    assert!(options.iter().any(|o| o.value == default), "{}.{}", g.id, p.key)
                }
            }
        }
    }
}

#[test]
fn inverse_trigonometry_is_accurate() {
    for i in 0..=4000 {
        let x = (i as f64 - 2000.0) * 0.0173;
        let u = (i as f64 - 2000.0) / 2000.0;
        assert!((dmath::atan(x) - x.atan()).abs() < 1e-12, "atan {x}");
        assert!((dmath::atan2(x, u) - x.atan2(u)).abs() < 1e-12, "atan2 {x} {u}");
        assert!((dmath::asin(u) - u.asin()).abs() < 1e-11, "asin {u}");
        assert!((dmath::acos(u) - u.acos()).abs() < 1e-11, "acos {u}");
        assert!((dmath::cbrt(x) - x.cbrt()).abs() < 1e-12 * x.abs().max(1.0), "cbrt {x}");
    }
}

#[test]
fn params_defaults_case_and_errors() {
    let g = get_generator("Metaballs").unwrap();
    let p = resolve_params(
        g,
        &[
            ("look".into(), Value::Str("GLASS".into())),
            ("balls".into(), Value::Num(4.0)),
        ],
    )
    .unwrap();
    assert_eq!(p.str("look"), "glass");
    assert_eq!(p.num("balls"), 4.0);
    assert_eq!(p.num("merge"), 0.55);
    assert!(resolve_params(g, &[("nope".into(), Value::Num(1.0))])
        .unwrap_err()
        .contains("unknown parameter"));
    assert!(resolve_params(g, &[("look".into(), Value::Str("plastic".into()))])
        .unwrap_err()
        .contains("look"));
    assert!(matches!(get_generator("nope"), Err(e) if e.contains("unknown generator")));
}
