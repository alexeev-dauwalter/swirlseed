# Changelog

## 0.2.0 — 2026-11-23

### Added

- Twelve new generators in the npm package, the Rust crate and the CLI, byte-identical between TypeScript and Rust:
  `sky`, `ebru`, `suminagashi`, `mesh-gradient`, `flow-field`, `watercolor`, `wet-on-wet`, `aurora`, `planets`,
  `dendritic-painting`, `paste-paper` and `metaballs` (3D: chrome, glass, glossy, lava, iridescent; 2D: layers,
  gradient, drops, contour, holo).
- npm: `GENERATORS`, `getGenerator`, `renderGenerator`, `resolveParams` and the browser helper `drawGenerator`.
- Rust: the `swirlseed::gen` module with the same API (`GENERATORS`, `get_generator`, `render_generator`,
  `resolve_params`) and parameter specs.
- CLI: `-g/--generator <id>`, repeatable `-p/--param key=value` and `--list`. With a generator a single `-s` number is
  the width and the height follows the generator's aspect ratio. The default output name is `<generator>-<seed>.png`.
- Deterministic `atan`, `atan2`, `asin`, `acos` and `cbrt` in `dmath`.
- `testdata/generators.txt`: 51 golden cases shared by both test suites.

### Unchanged

- The swirl pattern (`generate`) renders exactly the same bytes as in 0.1.0 (`golden.txt` is untouched).
