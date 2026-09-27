# PPT Header

A browser-based Rust app that extracts slide or page titles from `.pptx` and `.pdf` files and exports the selected results as CSV or Excel.

## Use the app

Open the deployed site: **[kazakan.github.io/ppt_header](https://kazakan.github.io/ppt_header/)**.

1. Choose a `.pptx` or `.pdf` file, or drag and drop it onto the upload area.
2. Review the extracted slide/page titles. Entries are selected by default; use the checkboxes to choose which ones to export.
3. Select **Export CSV** or **Export Excel** to download the selected entries.

The app runs in your browser; it does not require an account or a separate installation.

## What it does

- Upload a PowerPoint (`.pptx`) or PDF file
- Parse titles from each slide/page
- Review and selectively check the extracted entries
- Export selected rows to:
  - CSV
  - Excel (.xlsx)

This project runs in the browser via Yew + WebAssembly and is designed for GitHub Pages deployment.

## Project overview

- `src/main.rs` — UI, drag-and-drop upload flow, selection handling, CSV/Excel export
- `src/pptx_parser.rs` — extracts text titles from PPTX slide XML
- `src/pdf_parser.rs` — extracts page titles from PDF content and bookmarks
- `index.html` — app entry page for Trunk
- `.github/workflows/deploy.yml` — deployment to GitHub Pages

## Local development

### Prerequisites

- Rust toolchain
- `wasm32-unknown-unknown` target
- Trunk

### Install tools

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

### Run locally

```bash
trunk serve
```

Then open the local URL printed by Trunk in the browser.

## Production build

```bash
trunk build --release --public-url /ppt_header/
```

The built files are emitted to `dist/`.

## Notes for contributors

- Keep parsing logic browser-safe and WASM-compatible.
- Prefer small, targeted changes in the parser modules rather than rewriting the app shell.
- The app accepts `.pptx` and `.pdf` inputs only.
- CSV export includes a BOM for better Excel compatibility.

## Useful commands

```bash
cargo check
cargo build --target wasm32-unknown-unknown
trunk build --release
```

## Deployment

This repository includes a GitHub Actions workflow for GitHub Pages deployment. A push to `main` or `master` builds the Wasm app with Trunk and publishes the `dist/` directory. The deployed site is [https://kazakan.github.io/ppt_header/](https://kazakan.github.io/ppt_header/).

## Guidance for AI agents and contributors

### Repository layout

- `src/main.rs` — browser UI, file upload flow, selection toggles, CSV/Excel export
- `src/pptx_parser.rs` — parses `.pptx` files by reading slide XML and extracting title text
- `src/pdf_parser.rs` — parses PDF pages and bookmarks to infer titles
- `index.html` — app entry for Trunk
- `.github/workflows/deploy.yml` — GitHub Pages deployment pipeline

### Project constraints and expected behavior

- Keep the app compatible with `wasm32-unknown-unknown` and safe for browser execution; avoid server-only assumptions.
- Supported input types are `.pptx` and `.pdf` only.
- Preserve CSV/Excel export behavior, including the CSV BOM for Excel compatibility.
- Upload errors and parsing failures should be surfaced in the UI.
- Preserve the current Yew component flow and simple parser structure unless a feature requires a broader change.
- Prefer targeted, testable changes in the relevant UI or parser module. Avoid unrelated frameworks, server-side dependencies, and unnecessary abstractions.
- When changing parsing logic, validate both supported file formats.
- If deployment details change, ensure Trunk still emits `dist/` and the GitHub Pages base path remains correct.

### Useful checks

```bash
cargo check
cargo build --target wasm32-unknown-unknown
trunk build --release --public-url /ppt_header/
```
