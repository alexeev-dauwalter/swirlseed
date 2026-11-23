//! `swirlseed` — generate a fluid, glassy marbling pattern (or any of the other seeded generators:
//! sky, ebru, suminagashi, mesh gradient, flow field, watercolour, aurora, planets, metaballs, …) from a
//! string and save it as a PNG.

mod gen_cli;
mod util;

use std::error::Error;
use std::fs::File;
use std::io::{self, BufWriter, IsTerminal, Read, Write};
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, ValueEnum};
use swirlseed::gen::{get_generator, render_generator, resolve_params, Value};
use swirlseed::{
    create_pattern, palette_hex, render_pattern, DitherOptions, GrainOptions, HalftoneMode, HalftoneOptions,
    HalftoneShape, RenderOptions, RibsOptions,
};

const DEG: f64 = std::f64::consts::PI / 180.0;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ShapeArg {
    Dot,
    Square,
    Line,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum InkArg {
    /// Dots take the pattern colour on "paper"
    Color,
    /// A single ink on paper
    Mono,
}

/// Generate an image from a string: the fluid, glassy marbling pattern by default, or a sky, ebru,
/// suminagashi, mesh gradient, flow field, watercolour, aurora, planet, metaballs and more with -g.
/// The same seed always gives the same image (byte for byte, and identical to the `swirlseed` npm
/// package); change one character and you get a different one.
#[derive(Parser, Debug)]
#[command(name = "swirlseed", version, max_term_width = 100)]
#[command(after_help = "Examples:\n  swirlseed \"Hello, world!\"\n  swirlseed --list\n  \
swirlseed -g sky -p time=18.4 -p type=cumulus -s 1920x1080 sunset\n  swirlseed -g metaballs -p look=glass seed")]
struct Cli {
    /// Seed string; use "-" to read it from stdin
    #[arg(required_unless_present = "list")]
    seed: Option<String>,

    /// Output PNG path; "-" writes to stdout [default: <seed>.png, or <generator>-<seed>.png]
    #[arg(short, long, value_name = "FILE")]
    output: Option<String>,

    /// Image size: "1024" or "1920x1080" (16..8192 px per side). With a generator a single number is
    /// the width and the height follows the generator's aspect ratio
    #[arg(short, long, value_name = "SIZE", default_value = "1024", value_parser = util::parse_size_spec)]
    size: (u32, Option<u32>),

    /// What to draw: "swirl" (the default) or a generator id such as sky, ebru, metaballs (see --list)
    #[arg(short, long, value_name = "ID")]
    generator: Option<String>,

    /// A generator parameter, repeatable: -p time=18.5 -p type=cirrus (see --list)
    #[arg(short = 'p', long = "param", value_name = "KEY=VALUE", value_parser = gen_cli::parse_param)]
    params: Vec<(String, Value)>,

    /// List the generators and their parameters (with -g, only that one) and exit
    #[arg(long)]
    list: bool,

    /// Light and gloss strength, 0..1.5 [default: derived from the seed; 0 = flat]
    #[arg(long, value_name = "F")]
    shine: Option<f64>,

    /// Print the seed's mood, mode and palette to stderr
    #[arg(long)]
    info: bool,

    /// Enable reeded glass (any --ribs-* option enables it too)
    #[arg(long, help_heading = "Reeded glass")]
    ribs: bool,
    /// Number of flutes across the image height [default: 24]
    #[arg(long, value_name = "N", help_heading = "Reeded glass")]
    ribs_count: Option<f64>,
    /// Lens refraction strength, 0..1.5 [default: 0.6]
    #[arg(long, value_name = "F", help_heading = "Reeded glass")]
    ribs_strength: Option<f64>,
    /// Flute direction in degrees; 0 = vertical [default: 0]
    #[arg(long, value_name = "DEG", help_heading = "Reeded glass")]
    ribs_angle: Option<f64>,
    /// Shading of the flute faces, 0..1 [default: 0.5]
    #[arg(long, value_name = "F", help_heading = "Reeded glass")]
    ribs_shade: Option<f64>,

    /// Enable halftone (any --halftone-* option enables it too)
    #[arg(long, help_heading = "Halftone")]
    halftone: bool,
    /// Cell pitch in px for a 512 px image; scales with the image size [default: 10]
    #[arg(long, value_name = "PX", help_heading = "Halftone")]
    halftone_size: Option<f64>,
    /// Grid angle in degrees [default: 30]
    #[arg(long, value_name = "DEG", help_heading = "Halftone")]
    halftone_angle: Option<f64>,
    /// Dot shape [default: dot]
    #[arg(long, value_enum, value_name = "SHAPE", help_heading = "Halftone")]
    halftone_shape: Option<ShapeArg>,
    /// Ink [default: color]
    #[arg(long, value_enum, value_name = "INK", help_heading = "Halftone")]
    halftone_mode: Option<InkArg>,
    /// Glowing dots on a dark background instead of dark ink on light paper
    #[arg(long, help_heading = "Halftone")]
    halftone_invert: bool,
    /// Blend with the untouched pattern, 0..1 [default: 1]
    #[arg(long, value_name = "F", help_heading = "Halftone")]
    halftone_mix: Option<f64>,

    /// Enable ordered Bayer dithering (any --dither-* option enables it too)
    #[arg(long, help_heading = "Dither")]
    dither: bool,
    /// Levels per channel, 2..32 [default: 4]
    #[arg(long, value_name = "N", help_heading = "Dither")]
    dither_levels: Option<f64>,
    /// Size of one matrix cell in px for a 512 px image [default: 1.5]
    #[arg(long, value_name = "PX", help_heading = "Dither")]
    dither_scale: Option<f64>,
    /// Strength of the matrix pattern, 0..1 [default: 1]
    #[arg(long, value_name = "F", help_heading = "Dither")]
    dither_amount: Option<f64>,

    /// Enable film grain (any --grain-* option enables it too)
    #[arg(long, help_heading = "Grain")]
    grain: bool,
    /// Grain amount, 0..1 [default: 0.35]
    #[arg(long, value_name = "F", help_heading = "Grain")]
    grain_amount: Option<f64>,
    /// Grain size in px for a 512 px image [default: 1]
    #[arg(long, value_name = "PX", help_heading = "Grain")]
    grain_size: Option<f64>,
}

fn check(name: &str, v: Option<f64>, min: f64, max: f64) -> Result<(), String> {
    match v {
        Some(x) if !(x >= min && x <= max) => Err(format!("--{name} must be between {min} and {max}, got {x}")),
        _ => Ok(()),
    }
}

/// Range checks for the numeric options (the library itself accepts anything).
fn validate(c: &Cli) -> Result<(), String> {
    check("shine", c.shine, 0.0, 1.5)?;
    check("ribs-count", c.ribs_count, 1.0, 500.0)?;
    check("ribs-strength", c.ribs_strength, 0.0, 1.5)?;
    check("ribs-angle", c.ribs_angle, -360.0, 360.0)?;
    check("ribs-shade", c.ribs_shade, 0.0, 1.0)?;
    check("halftone-size", c.halftone_size, 2.0, 200.0)?;
    check("halftone-angle", c.halftone_angle, -360.0, 360.0)?;
    check("halftone-mix", c.halftone_mix, 0.0, 1.0)?;
    check("dither-levels", c.dither_levels, 2.0, 32.0)?;
    check("dither-scale", c.dither_scale, 1.0, 64.0)?;
    check("dither-amount", c.dither_amount, 0.0, 1.0)?;
    check("grain-amount", c.grain_amount, 0.0, 1.0)?;
    check("grain-size", c.grain_size, 1.0, 64.0)?;
    Ok(())
}

fn build_options(c: &Cli) -> RenderOptions {
    let ribs = (c.ribs
        || c.ribs_count.is_some()
        || c.ribs_strength.is_some()
        || c.ribs_angle.is_some()
        || c.ribs_shade.is_some())
    .then(|| RibsOptions {
        count: c.ribs_count,
        strength: c.ribs_strength,
        angle: c.ribs_angle.map(|d| d * DEG),
        shade: c.ribs_shade,
    });

    let halftone = (c.halftone
        || c.halftone_size.is_some()
        || c.halftone_angle.is_some()
        || c.halftone_shape.is_some()
        || c.halftone_mode.is_some()
        || c.halftone_invert
        || c.halftone_mix.is_some())
    .then(|| HalftoneOptions {
        size: c.halftone_size,
        angle: c.halftone_angle.map(|d| d * DEG),
        shape: c.halftone_shape.map(|s| match s {
            ShapeArg::Dot => HalftoneShape::Dot,
            ShapeArg::Square => HalftoneShape::Square,
            ShapeArg::Line => HalftoneShape::Line,
        }),
        mode: c.halftone_mode.map(|m| match m {
            InkArg::Color => HalftoneMode::Color,
            InkArg::Mono => HalftoneMode::Mono,
        }),
        invert: c.halftone_invert.then_some(true),
        mix: c.halftone_mix,
    });

    let dither =
        (c.dither || c.dither_levels.is_some() || c.dither_scale.is_some() || c.dither_amount.is_some()).then(|| {
            DitherOptions {
                levels: c.dither_levels,
                scale: c.dither_scale,
                amount: c.dither_amount,
            }
        });

    let grain = (c.grain || c.grain_amount.is_some() || c.grain_size.is_some()).then(|| GrainOptions {
        amount: c.grain_amount,
        size: c.grain_size,
    });

    RenderOptions {
        shine: c.shine,
        ribs,
        halftone,
        dither,
        grain,
    }
}

fn write_png(out: impl Write, w: u32, h: u32, rgba: &[u8]) -> Result<(), Box<dyn Error>> {
    // the image is fully opaque, so store plain RGB (smaller files)
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for px in rgba.chunks_exact(4) {
        rgb.extend_from_slice(&px[..3]);
    }
    let mut encoder = png::Encoder::new(out, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&rgb)?;
    writer.finish()?;
    Ok(())
}

/// True when any option of the swirl pattern was given.
fn swirl_options_used(o: &RenderOptions) -> bool {
    o.shine.is_some() || o.ribs.is_some() || o.halftone.is_some() || o.dither.is_some() || o.grain.is_some()
}

fn save(cli: &Cli, default_path: String, w: u32, h: u32, rgba: &[u8]) -> Result<(), Box<dyn Error>> {
    if cli.output.as_deref() == Some("-") {
        write_png(BufWriter::new(io::stdout().lock()), w, h, rgba)?;
    } else {
        let path = cli.output.clone().unwrap_or(default_path);
        write_png(BufWriter::new(File::create(&path)?), w, h, rgba)?;
        eprintln!("wrote {path} ({w}x{h})");
    }
    Ok(())
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    if cli.list {
        print!("{}", gen_cli::list(cli.generator.as_deref())?);
        return Ok(());
    }
    let seed = match cli.seed.as_deref() {
        Some("-") => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            util::strip_trailing_newline(buf)
        }
        Some(s) => s.to_owned(),
        None => return Err("a seed is required".into()),
    };

    let to_stdout = cli.output.as_deref() == Some("-");
    if to_stdout && io::stdout().is_terminal() {
        return Err("refusing to write a PNG to a terminal; redirect stdout or pass -o FILE".into());
    }

    let options = build_options(&cli);
    let gid = cli.generator.clone().unwrap_or_else(|| gen_cli::SWIRL.to_owned());
    if !gid.eq_ignore_ascii_case(gen_cli::SWIRL) {
        let g = get_generator(&gid)?;
        if swirl_options_used(&options) {
            return Err(format!(
                "--shine, --ribs*, --halftone*, --dither* and --grain* only apply to the swirl pattern; \
                 {} takes -p key=value (see swirlseed --list -g {})",
                g.id, g.id
            )
            .into());
        }
        gen_cli::check_ranges(g, &cli.params)?;
        let resolved = resolve_params(g, &cli.params)?;
        let (w, h) = gen_cli::resolve_size(cli.size, Some(g.aspect));
        if cli.info {
            eprintln!("seed       {seed:?}");
            eprintln!("generator  {} ({})", g.id, g.title);
            eprintln!("size       {w}x{h}");
            for (k, v) in &resolved.0 {
                match v {
                    Value::Num(n) => eprintln!("{k:<10} {n}"),
                    Value::Str(s) => eprintln!("{k:<10} {s}"),
                }
            }
        }
        let rgba = render_generator(g.id, &seed, w, h, &cli.params)?;
        return save(&cli, format!("{}-{}.png", g.id, util::slug(&seed)), w, h, &rgba);
    }
    if !cli.params.is_empty() {
        return Err("-p/--param only applies to a generator; pick one with -g (see swirlseed --list)".into());
    }

    let (w, h) = gen_cli::resolve_size(cli.size, None);
    let params = create_pattern(&seed);

    if cli.info {
        eprintln!("seed     {seed:?}");
        eprintln!("size     {w}x{h}");
        eprintln!("mood     {}", params.mood.as_str());
        eprintln!("mode     {}", params.mode.as_str());
        eprintln!(
            "shine    {:.2}{}",
            options.shine.unwrap_or(params.shine),
            if options.shine.is_some() { "" } else { " (from seed)" }
        );
        eprintln!("palette  {}", palette_hex(&params).join(" "));
    }

    let rgba = render_pattern(&params, w, h, &options);
    save(&cli, format!("{}.png", util::slug(&seed)), w, h, &rgba)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Err(msg) = validate(&cli) {
        Cli::command().error(ErrorKind::ValueValidation, msg).exit();
    }
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
