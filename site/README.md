# swirlseed site

Static demo site: a catalogue of every generator with one shared seed, a page per generator and glass-UI examples. No framework and no runtime dependencies. The pages import the library's compiled ES modules directly (rendering in module Web Workers), so the demo always runs the same code that is published to npm.

## Build

```sh
npm --prefix ../packages/swirlseed install
npm --prefix ../packages/swirlseed run build
node build.mjs          # writes site/dist
```

Preview locally with any static server, for example `npx serve dist` (ES modules do not load from `file://`).

## Cloudflare Pages

The `swirlseed` Pages project is connected to GitHub and deploys automatically when `main` is pushed. The build settings are stored in Cloudflare Pages.

| Setting | Value |
| --- | --- |
| Production branch | `main` |
| Root directory | *(repository root)* |
| Build command | `npm --prefix packages/swirlseed install && npm --prefix packages/swirlseed run build && node site/build.mjs` |
| Build output directory | `site/dist` |

To redeploy the current remote `main` with the Cloudflare CLI:

```sh
cf pages deployments create swirlseed --branch main
```

Cloudflare builds the committed files from GitHub using the settings above.

`src/_headers` sets security and cache headers.

## Layout

| path | what |
| --- | --- |
| `src/index.html` | home: generator catalogue (shared seed, live previews), examples, install |
| `src/swirl/index.html` | the swirl generator with every effect |
| `src/gen.html` | template for the other generator pages; `build.mjs` writes `dist/<id>/index.html` for each entry of `GENERATORS` |
| `src/app/` | page scripts: `catalog.js`, `gen-page.js` (controls built from the parameter spec), `pool.js` and `worker.js` (rendering off the main thread), `code.js` (snippet highlighting) |
| `src/style.css` | shared styles |

Pages keep `?seed=…` and the changed parameters in the URL, so any image can be shared as a link.
