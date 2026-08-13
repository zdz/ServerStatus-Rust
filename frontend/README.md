# ServerStatus-Rust Frontend

This directory contains the maintainable React/TypeScript source for the default ServerStatus-Rust UI.

## Requirements

- Node.js `^20.19.0` or `>=22.12.0`
- npm

## Development

```bash
cd frontend
npm install
npm run dev
```

The development server expects the ServerStatus JSON endpoint at `json/stats.json`. Use the production build for the embedded Server deployment path.

## Tests

```bash
npm test
```

## Production build

```bash
npm run build
```

The Vite build writes directly to `../web` and preserves existing static resources such as flags, OS icons, favicons and Jinja templates. `web/index.html` and `web/asset-manifest.json` are regenerated on each build.

The Rust Server embeds the complete `web/` directory at compile time, so rebuild the frontend before compiling the Server binary whenever frontend source changes.
