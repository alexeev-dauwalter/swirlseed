//! Seeded generators beyond the swirl pattern: sky, marbling, paint, light and 3D.
//! Every file mirrors its TypeScript twin in `packages/swirlseed/src/gen/` expression by expression,
//! which keeps the two byte-identical; a few lints that would reshape those expressions are off here.

#![allow(
    clippy::neg_cmp_op_on_partial_ord,
    clippy::manual_range_contains,
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::needless_range_loop
)]

mod aurora;
pub mod core;
mod flow;
mod marble;
mod mesh;
mod metaballs;
mod paint;
mod planets;
pub mod raster;
mod sky;
pub mod spec;

pub use spec::{resolve_params, Generator, Kind, Opt, Param, Params, Value};

/// Every generator, in catalogue order.
pub const GENERATORS: &[Generator] = &[
    sky::SKY,
    marble::EBRU,
    marble::SUMINAGASHI,
    mesh::MESH,
    flow::FLOW,
    paint::WATERCOLOR,
    paint::WET_ON_WET,
    aurora::AURORA,
    planets::PLANETS,
    paint::DENDRITIC,
    paint::PASTE_PAPER,
    metaballs::METABALLS,
];

pub fn get_generator(id: &str) -> Result<&'static Generator, String> {
    let l = id.to_lowercase();
    GENERATORS.iter().find(|g| g.id == l).ok_or_else(|| {
        format!(
            "unknown generator \"{id}\"; available: {}",
            GENERATORS.iter().map(|g| g.id).collect::<Vec<_>>().join(", ")
        )
    })
}

/// Render a generator to RGBA bytes (`width * height * 4`). Missing parameters take their defaults.
pub fn render_generator(
    id: &str,
    seed: &str,
    width: u32,
    height: u32,
    params: &[(String, Value)],
) -> Result<Vec<u8>, String> {
    let g = get_generator(id)?;
    let p = resolve_params(g, params)?;
    Ok((g.render)(seed, width as usize, height as usize, &p))
}
