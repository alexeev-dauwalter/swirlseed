//! Parameter descriptions shared by the API and the CLI. Mirrors `spec.ts`.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num(f64),
    Str(String),
}

#[derive(Clone, Copy, Debug)]
pub struct Opt {
    pub value: &'static str,
    pub label: &'static str,
    pub group: &'static str,
}
pub const fn opt(value: &'static str, label: &'static str) -> Opt {
    Opt {
        value,
        label,
        group: "",
    }
}
pub const fn opt_g(value: &'static str, label: &'static str, group: &'static str) -> Opt {
    Opt { value, label, group }
}

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Number {
        min: f64,
        max: f64,
        step: f64,
        default: f64,
        unit: &'static str,
    },
    Choice {
        default: &'static str,
        options: &'static [Opt],
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Param {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: Kind,
}
pub const fn number(
    key: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    step: f64,
    default: f64,
    unit: &'static str,
) -> Param {
    Param {
        key,
        label,
        kind: Kind::Number {
            min,
            max,
            step,
            default,
            unit,
        },
    }
}
pub const fn choice(key: &'static str, label: &'static str, default: &'static str, options: &'static [Opt]) -> Param {
    Param {
        key,
        label,
        kind: Kind::Choice { default, options },
    }
}

pub type RenderFn = fn(&str, usize, usize, &Params) -> Vec<u8>;

/// One generator: its id, description, parameters and renderer.
#[derive(Clone, Copy)]
pub struct Generator {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub aspect: f64,
    pub params: &'static [Param],
    pub render: RenderFn,
}

/// Resolved parameter values (defaults filled in).
#[derive(Clone, Debug, Default)]
pub struct Params(pub BTreeMap<String, Value>);

impl Params {
    pub fn num(&self, k: &str) -> f64 {
        match self.0.get(k) {
            Some(Value::Num(v)) => *v,
            _ => panic!("missing number parameter {k}"),
        }
    }
    pub fn str(&self, k: &str) -> &str {
        match self.0.get(k) {
            Some(Value::Str(v)) => v,
            _ => panic!("missing choice parameter {k}"),
        }
    }
}

/// Fill in defaults and check values. Choice values match case-insensitively.
pub fn resolve_params(g: &Generator, given: &[(String, Value)]) -> Result<Params, String> {
    for (k, _) in given {
        if !g.params.iter().any(|p| p.key == k) {
            return Err(format!("{}: unknown parameter \"{}\"", g.id, k));
        }
    }
    let mut out = BTreeMap::new();
    for p in g.params {
        let v = given.iter().rev().find(|(k, _)| k == p.key).map(|(_, v)| v.clone());
        match p.kind {
            Kind::Number { default, .. } => {
                let n = match v {
                    None => default,
                    Some(Value::Num(n)) => n,
                    Some(Value::Str(s)) => s
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| format!("{}: \"{}\" must be a number", g.id, p.key))?,
                };
                if !n.is_finite() {
                    return Err(format!("{}: \"{}\" must be a number", g.id, p.key));
                }
                out.insert(p.key.to_string(), Value::Num(n));
            }
            Kind::Choice { default, options } => {
                let s = match v {
                    None => default.to_string(),
                    Some(Value::Num(n)) => n.to_string(),
                    Some(Value::Str(s)) => s,
                };
                let sl = s.to_lowercase();
                let o = options.iter().find(|o| o.value.to_lowercase() == sl).ok_or_else(|| {
                    format!(
                        "{}: \"{}\" must be one of {}",
                        g.id,
                        p.key,
                        options.iter().map(|o| o.value).collect::<Vec<_>>().join(", ")
                    )
                })?;
                out.insert(p.key.to_string(), Value::Str(o.value.to_string()));
            }
        }
    }
    Ok(Params(out))
}
