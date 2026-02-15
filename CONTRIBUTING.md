# Contributing to Zentra

Thanks for contributing to Zentra. This project is community-built and focused on fast, reliable voice dictation.

## Ways to contribute

- Report bugs via GitHub Issues
- Request new features
- Improve docs and onboarding
- Submit code improvements with tests
- Share feedback from real usage

## Development setup

### Prerequisites

- Node.js 18+
- Rust stable toolchain
- Tauri CLI (`npm i -D @tauri-apps/cli`)
- Windows 10/11 for full packaging validation

### Run locally

```bash
git clone https://github.com/DaviBonetto/zentra.git
cd zentra
npm install
npm run tauri:dev
```

### Build locally

```bash
npm run build
npm run tauri:build
```

## Commit convention (required)

Use Conventional Commits:

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation-only changes
- `refactor:` internal restructuring without behavior change
- `perf:` performance improvement
- `test:` tests added/updated
- `chore:` tooling/maintenance

Examples:

- `feat(paste): add safer target-window validation`
- `fix(ui): prevent toast clipping on compact bar`
- `docs: update setup screenshots in README`

## Code quality

### Frontend

- Keep TypeScript strictness intact
- Keep component logic readable and composable
- Run lint/typecheck before PR (if configured)

### Rust backend

- Keep modules cohesive and explicit
- Preserve current command contracts
- Run formatting/checks:

```bash
cargo fmt --all
cargo check
```

## Pull request checklist

Before opening a PR, ensure:

- [ ] Scope is focused and clearly described
- [ ] Tested on Windows in `npm run tauri:dev`
- [ ] `npm run build` succeeds
- [ ] `cargo check` succeeds
- [ ] No API keys, local configs, or secrets included
- [ ] CHANGELOG entry added (if user-facing)
- [ ] Screenshots included for visible UI changes

## Reporting bugs

Use the bug template and include:

- Steps to reproduce
- Expected vs actual behavior
- Logs/screenshots
- OS and app version

## Thank you

Every contribution helps Zentra become the best open-source voice dictation app.
