# swirlseed site

Static demo site: a live generator and glass-UI examples. No framework and no runtime dependencies. The page imports the library's compiled ES modules directly, so the demo always runs the same code that is published to npm.

## Build

```sh
npm --prefix ../packages/swirlseed install
npm --prefix ../packages/swirlseed run build
node build.mjs          # writes site/dist
```

Preview locally with any static server, for example `npx serve dist` (ES modules do not load from `file://`).

## Cloudflare Pages

Connect the GitHub repository in the Pages dashboard and use:

| Setting | Value |
| --- | --- |
| Production branch | `main` |
| Root directory | *(repository root)* |
| Build command | `npm --prefix packages/swirlseed install && npm --prefix packages/swirlseed run build && node site/build.mjs` |
| Build output directory | `site/dist` |

Or deploy a local build directly: `cd site && npx wrangler pages deploy dist --project-name swirlseed`.

`src/_headers` sets security and cache headers; the page itself is a single `index.html`.
