//! Seed hashing and the PRNG. Mirrors `rng.ts`.

/// cyrb128: fast 128-bit string hash with a strong avalanche effect (1 char changes every bit).
/// Takes UTF-16 code units, exactly like `String.prototype.charCodeAt` in JavaScript.
pub fn cyrb128(units: impl Iterator<Item = u16>) -> [u32; 4] {
    let mut h1: u32 = 1779033703;
    let mut h2: u32 = 3144134277;
    let mut h3: u32 = 1013904242;
    let mut h4: u32 = 2773480762;
    for k in units {
        let k = k as u32;
        h1 = h2 ^ (h1 ^ k).wrapping_mul(597399067);
        h2 = h3 ^ (h2 ^ k).wrapping_mul(2869860233);
        h3 = h4 ^ (h3 ^ k).wrapping_mul(951274213);
        h4 = h1 ^ (h4 ^ k).wrapping_mul(2716044179);
    }
    h1 = (h3 ^ (h1 >> 18)).wrapping_mul(597399067);
    h2 = (h4 ^ (h2 >> 22)).wrapping_mul(2869860233);
    h3 = (h1 ^ (h3 >> 17)).wrapping_mul(951274213);
    h4 = (h2 ^ (h4 >> 19)).wrapping_mul(2716044179);
    h1 ^= h2 ^ h3 ^ h4;
    h2 ^= h1;
    h3 ^= h1;
    h4 ^= h1;
    [h1, h2, h3, h4]
}

/// sfc32: small, fast PRNG with 128-bit state. Yields numbers in [0, 1).
pub struct Rng {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

impl Rng {
    pub fn new(a: u32, b: u32, c: u32, d: u32) -> Self {
        Rng { a, b, c, d }
    }

    pub fn next_f64(&mut self) -> f64 {
        let mut t = self.a.wrapping_add(self.b);
        self.a = self.b ^ (self.b >> 9);
        self.b = self.c.wrapping_add(self.c << 3);
        self.c = self.c.rotate_left(21);
        self.d = self.d.wrapping_add(1);
        t = t.wrapping_add(self.d);
        self.c = self.c.wrapping_add(t);
        t as f64 / 4294967296.0
    }

    /// `a + (b - a) * next()`
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next_f64()
    }
}

#[cfg(feature = "nfc")]
fn normalize(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfc().collect()
}

/// Without the `nfc` feature the seed is used as is (identical for ASCII and precomposed text).
#[cfg(not(feature = "nfc"))]
fn normalize(s: &str) -> String {
    s.to_owned()
}

/// An independent random stream per aspect of the pattern ("geo", "pal", "noise").
pub fn make_rng(seed: &str, stream: &str) -> Rng {
    let seed = normalize(seed);
    let units = seed
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .chain(stream.encode_utf16());
    let [a, b, c, d] = cyrb128(units);
    let mut rng = Rng::new(a, b, c, d);
    for _ in 0..15 {
        rng.next_f64(); // warm-up
    }
    rng
}
