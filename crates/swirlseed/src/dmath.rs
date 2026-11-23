//! Deterministic math.
//!
//! `f64::sin`, `f64::exp`, `f64::powf`, ... call the platform libm and may differ in the last bits
//! from other platforms and from JavaScript engines. To get byte-identical output everywhere,
//! swirlseed uses only these functions, which are built from `+ - * /`, `floor` and `sqrt`
//! (all exactly specified by IEEE 754). The TypeScript package contains the same code in
//! `dmath.ts`; keep the two files in sync, expression by expression.

use std::sync::OnceLock;

pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = std::f64::consts::TAU;
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
const LN2: f64 = std::f64::consts::LN_2;

struct Coeffs {
    sin: [f64; 12],
    cos: [f64; 13],
    exp: [f64; 16],
    ln: [f64; 13],
}

fn coeffs() -> &'static Coeffs {
    static C: OnceLock<Coeffs> = OnceLock::new();
    C.get_or_init(|| {
        // 1/n! for n = 0..32, computed identically in every language.
        let mut inv_fact = [0.0f64; 33];
        inv_fact[0] = 1.0;
        let mut f = 1.0f64;
        for i in 1..=32usize {
            f *= i as f64;
            inv_fact[i] = 1.0 / f;
        }
        let mut c = Coeffs {
            sin: [0.0; 12],
            cos: [0.0; 13],
            exp: [0.0; 16],
            ln: [0.0; 13],
        };
        for k in 0..12usize {
            c.sin[k] = if k % 2 == 1 {
                -inv_fact[2 * k + 1]
            } else {
                inv_fact[2 * k + 1]
            };
        }
        for k in 0..13usize {
            c.cos[k] = if k % 2 == 1 { -inv_fact[2 * k] } else { inv_fact[2 * k] };
            c.ln[k] = 1.0 / (2 * k + 1) as f64;
        }
        c.exp.copy_from_slice(&inv_fact[..16]);
        c
    })
}

#[inline]
fn horner(c: &[f64], x: f64) -> f64 {
    let mut p = c[c.len() - 1];
    for i in (0..c.len() - 1).rev() {
        p = p * x + c[i];
    }
    p
}

/// x reduced to [-pi, pi).
#[inline]
fn reduce(x: f64) -> f64 {
    let k = (x / TAU + 0.5).floor();
    x - k * TAU
}

pub fn sin(x: f64) -> f64 {
    let mut r = reduce(x);
    if r > HALF_PI {
        r = PI - r;
    } else if r < -HALF_PI {
        r = -PI - r;
    }
    r * horner(&coeffs().sin, r * r)
}

pub fn cos(x: f64) -> f64 {
    let mut r = reduce(x).abs();
    let mut s = 1.0;
    if r > HALF_PI {
        r = PI - r;
        s = -1.0;
    }
    s * horner(&coeffs().cos, r * r)
}

/// Exact 2^k for integer k (binary exponentiation of an exact power of two).
fn pow2i(k: f64) -> f64 {
    let mut n = k.abs() as u32;
    let mut base = if k < 0.0 { 0.5 } else { 2.0 };
    let mut r = 1.0;
    while n > 0 {
        if n & 1 == 1 {
            r *= base;
        }
        base *= base;
        n >>= 1;
    }
    r
}

pub fn exp(x: f64) -> f64 {
    if x < -745.0 {
        return 0.0;
    }
    if x > 709.0 {
        return f64::INFINITY;
    }
    let k = (x / LN2 + 0.5).floor();
    let r = x - k * LN2;
    horner(&coeffs().exp, r) * pow2i(k)
}

/// Natural logarithm for finite x > 0.
pub fn ln(mut x: f64) -> f64 {
    let mut e: i32 = 0;
    while x >= 1.5 {
        x *= 0.5;
        e += 1;
    }
    while x < 0.75 {
        x *= 2.0;
        e -= 1;
    }
    let s = (x - 1.0) / (x + 1.0);
    2.0 * s * horner(&coeffs().ln, s * s) + e as f64 * LN2
}

/// x^y for x >= 0 (0 for x <= 0).
pub fn pow(x: f64, y: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    exp(y * ln(x))
}

/// Round half up (like JavaScript's `Math.round`).
#[inline]
pub fn round_half_up(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn atan_c() -> &'static [f64; 14] {
    static C: OnceLock<[f64; 14]> = OnceLock::new();
    C.get_or_init(|| {
        let mut c = [0.0f64; 14];
        for k in 0..14usize {
            c[k] = (if k % 2 == 1 { -1.0 } else { 1.0 }) / (2 * k + 1) as f64;
        }
        c
    })
}

/// Arctangent, built from the series after two half-angle reductions (|x| <= tan(pi/16)).
pub fn atan(mut x: f64) -> f64 {
    let mut s = 1.0;
    if x < 0.0 {
        x = -x;
        s = -1.0;
    }
    let mut off = false;
    if x > 1.0 {
        x = 1.0 / x;
        off = true;
    }
    x = x / (1.0 + (1.0 + x * x).sqrt());
    x = x / (1.0 + (1.0 + x * x).sqrt());
    let mut r = 4.0 * x * horner(atan_c(), x * x);
    if off {
        r = HALF_PI - r;
    }
    s * r
}

/// atan2(y, x) in (-pi, pi].
pub fn atan2(y: f64, x: f64) -> f64 {
    if x > 0.0 {
        return atan(y / x);
    }
    if x < 0.0 {
        return if y >= 0.0 { atan(y / x) + PI } else { atan(y / x) - PI };
    }
    if y > 0.0 {
        return HALF_PI;
    }
    if y < 0.0 {
        return -HALF_PI;
    }
    0.0
}

pub fn asin(x: f64) -> f64 {
    let x = if x < -1.0 {
        -1.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    };
    atan2(x, (1.0 - x * x).sqrt())
}

pub fn acos(x: f64) -> f64 {
    let x = if x < -1.0 {
        -1.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    };
    atan2((1.0 - x * x).sqrt(), x)
}

/// Real cube root (sign kept).
pub fn cbrt(x: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    if x < 0.0 {
        -pow(-x, 1.0 / 3.0)
    } else {
        pow(x, 1.0 / 3.0)
    }
}
