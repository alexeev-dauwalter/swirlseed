//! Generator selection for the CLI: `--generator`, `--param key=value` and `--list`
//! (dependency-free and unit-tested).

use swirlseed::gen::{get_generator, Generator, Kind, Value, GENERATORS};

use crate::util::{MAX_SIDE, MIN_SIDE};

/// The original marbling pattern; its options are the regular `--shine`, `--ribs-*`, … flags.
pub const SWIRL: &str = "swirl";

/// Parses one `--param key=value`. Numbers become numbers, anything else stays text (choice values).
pub fn parse_param(s: &str) -> Result<(String, Value), String> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| format!("`{s}` is not key=value, e.g. -p time=18.5 or -p look=glass"))?;
    let (k, v) = (k.trim(), v.trim());
    if k.is_empty() || v.is_empty() {
        return Err(format!("`{s}` is not key=value, e.g. -p time=18.5 or -p look=glass"));
    }
    let value = match v.parse::<f64>() {
        Ok(n) if n.is_finite() => Value::Num(n),
        _ => Value::Str(v.to_owned()),
    };
    Ok((k.to_owned(), value))
}

/// Final image size. An explicit `WxH` is kept; a single number is a square for the swirl pattern and
/// the width for a generator, whose height then follows its natural aspect ratio.
pub fn resolve_size(size: (u32, Option<u32>), aspect: Option<f64>) -> (u32, u32) {
    match (size, aspect) {
        ((w, Some(h)), _) => (w, h),
        ((w, None), Some(a)) if a > 0.0 => {
            let h = (w as f64 / a + 0.5).floor() as u32;
            (w, h.clamp(MIN_SIDE, MAX_SIDE))
        }
        ((w, None), _) => (w, w),
    }
}

/// Checks every `--param` against the generator's ranges (the library itself accepts any number).
pub fn check_ranges(g: &Generator, params: &[(String, Value)]) -> Result<(), String> {
    for (k, v) in params {
        let Some(p) = g.params.iter().find(|p| p.key == k) else {
            continue; // reported as unknown by resolve_params
        };
        if let (Kind::Number { min, max, .. }, Value::Num(n)) = (&p.kind, v) {
            if !(*n >= *min && *n <= *max) {
                return Err(format!(
                    "{}: {k} must be between {} and {}, got {n}",
                    g.id,
                    fmt_num(*min),
                    fmt_num(*max)
                ));
            }
        }
    }
    Ok(())
}

fn fmt_num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

/// Human-readable description of one generator and its parameters.
pub fn describe(g: &Generator) -> String {
    let mut out = format!("{}  ({})\n  {}\n", g.id, g.title, g.description);
    for p in g.params {
        match &p.kind {
            Kind::Number {
                min,
                max,
                step,
                default,
                unit,
            } => {
                let unit = match *unit {
                    "" => String::new(),
                    "%" => " (fraction)".to_owned(),
                    "x" => " (multiplier)".to_owned(),
                    u => format!(" ({u})"),
                };
                out += &format!(
                    "  -p {}=<{}..{}>{}  {}, step {}, default {}\n",
                    p.key,
                    fmt_num(*min),
                    fmt_num(*max),
                    unit,
                    p.label,
                    fmt_num(*step),
                    fmt_num(*default)
                );
            }
            Kind::Choice { default, options } => {
                let mut groups: Vec<&str> = Vec::new();
                for o in options.iter() {
                    if !o.group.is_empty() && !groups.contains(&o.group) {
                        groups.push(o.group);
                    }
                }
                let list = if groups.is_empty() {
                    options.iter().map(|o| o.value).collect::<Vec<_>>().join(" | ")
                } else {
                    groups
                        .iter()
                        .map(|gr| {
                            let vs: Vec<&str> = options.iter().filter(|o| o.group == *gr).map(|o| o.value).collect();
                            format!("{gr}: {}", vs.join(" | "))
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                out += &format!("  -p {}=<{}>  {}, default {}\n", p.key, list, p.label, default);
            }
        }
    }
    out
}

/// Text for `--list` (all generators) or `--list -g <id>` (one).
pub fn list(only: Option<&str>) -> Result<String, String> {
    match only {
        Some(id) if !id.eq_ignore_ascii_case(SWIRL) => Ok(describe(get_generator(id)?)),
        Some(_) => Ok(swirl_text()),
        None => {
            let mut out = swirl_text();
            for g in GENERATORS {
                out.push('\n');
                out += &describe(g);
            }
            Ok(out)
        }
    }
}

fn swirl_text() -> String {
    format!(
        "{SWIRL}  (Swirl, the default)\n  Fluid, glassy marbling. Options: --shine, --ribs-*, --halftone-*, --dither-*, --grain-* (see --help).\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params() {
        assert!(matches!(parse_param("time=18.5"), Ok((k, Value::Num(v))) if k == "time" && v == 18.5));
        assert!(matches!(parse_param(" look = glass "), Ok((k, Value::Str(v))) if k == "look" && v == "glass"));
        assert!(matches!(parse_param("sun=-55"), Ok((_, Value::Num(v))) if v == -55.0));
        assert!(parse_param("look").is_err());
        assert!(parse_param("=3").is_err());
        assert!(parse_param("a=").is_err());
    }

    #[test]
    fn sizes() {
        assert_eq!(resolve_size((1024, None), None), (1024, 1024));
        assert_eq!(resolve_size((1600, None), Some(1.6)), (1600, 1000));
        assert_eq!(resolve_size((1500, None), Some(1.5)), (1500, 1000));
        assert_eq!(resolve_size((800, Some(800)), Some(1.5)), (800, 800));
        assert_eq!(resolve_size((16, None), Some(1.6)), (16, 16));
    }

    #[test]
    fn ranges() {
        let g = get_generator("sky").unwrap();
        assert!(check_ranges(g, &[("time".into(), Value::Num(12.0))]).is_ok());
        assert!(check_ranges(g, &[("time".into(), Value::Num(25.0))]).is_err());
        assert!(check_ranges(g, &[("type".into(), Value::Str("cirrus".into()))]).is_ok());
    }

    #[test]
    fn listing() {
        let all = list(None).unwrap();
        for g in GENERATORS {
            assert!(all.contains(g.id), "{}", g.id);
        }
        let mb = list(Some("metaballs")).unwrap();
        assert!(mb.contains("3D: chrome | glass"), "{mb}");
        assert!(mb.contains("2D: layers"), "{mb}");
        assert!(list(Some("nope")).is_err());
        assert!(list(Some("swirl")).unwrap().contains("--ribs"));
    }
}
