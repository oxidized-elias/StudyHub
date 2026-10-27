# Development and release

## Layout and dependencies

The Cargo workspace contains notes (`notes-web`, library), learning (`selfhosted-ai-tests`, library), and the integrated executable (`study-hub`). Only the executable binds a socket. The historical package names are internal module identities. Run commands from the repository unless noted; the Node launcher itself resolves paths independently of the current directory.

Runtime dependencies are Node 22+, a built native executable, Poppler for PDFs, browser CDN access, and your configured AI endpoint. Building requires stable Rust/Cargo and native C build tooling for bundled SQLite. Cargo.lock is committed and the launcher builds with `--locked`. Node has zero runtime packages; package-lock.json records that. UI test runtimes are optional development tools, never required to serve the application.

## Commands

```bash
npm test                            # launcher and all Rust tests
cargo fmt --all -- --check
cargo test --workspace --locked --offline  # with a populated Cargo cache
node server.mjs --build-only         # debug binary for smoke tests
node server.mjs --release --build-only
node server.mjs --release --no-build
python3 scripts/document-api.py --check
```

For the sixteen UI/parser checks:

```bash
npm install --no-save --package-lock=false typescript@5.8.3 marked@12.0.2 katex@0.16.11
npm run test:ui
```

The scripts alternatively accept absolute CommonJS runtime paths through `TYPESCRIPT_RUNTIME`, `MARKED_RUNTIME`, `KATEX_RUNTIME`. They compile JSX and run focused component/parser harnesses. `tests/rich_input.cjs` is an additional optional Chromium DOM test; it needs a working Chromium environment. The harnesses do not prove pixel layout or browser accessibility.

```bash
python3 tests/smoke.py
python3 tests/suite_smoke.py
```

These integration smoke tests start the integrated executable, fake Ollama and temporary data directories. They need socket permission and make no paid model calls. The suite test covers source approval/import, generation, attempts/scoring, flashcards, revisions, sharing, tools and history. The socket-free Rust server tests additionally exercise real notes/learning initialization and in-memory integration, so routing/authentication can be verified in restricted environments.

## Test map

- Rust notes tests: recovery, review durability, precise file ownership, style overrides, internal links and one-note log clearing.
- Rust learning tests: provider adapters, citation normalization, source restrictions, attempts/answer visibility, timers, tools/canvas, cancellation, partial batches, history deletion, corrections, journals, recall and mastery scheduling.
- Rust server tests: route prefixes/HTML injection, authentication on every module and internal notes dispatch.
- `launcher.test.mjs`: flags, paths, one binary build and help without side effects.
- `ui.cjs`, `suite_ui.cjs`: upload/review behaviors, all JSX entry points, iframe handoff, canvas channels and empty states.
- `request_audit.cjs`, `experience.cjs`, `keyboard.cjs`, `study_workflows.cjs`: input/attempt/tutorial/reference contracts.
- `history_delete.cjs`, `model_select.cjs`, `warning_dismiss.cjs`, `live_logs.cjs`, `task_controls.cjs`, `notifications.cjs`, `storage_reset.cjs`: scoped controls and lifecycle regressions.
- `render.cjs`, `learning_render.cjs`: real Marked/KaTeX rendering, code protection, citations and stored math repair.

## Changes

Add routes to the owning library and regenerate `docs/API.md` with `python3 scripts/document-api.py`. Keep the application map current when moving modules. Preserve ownership boundaries: Learn calls the Notes router rather than querying its database. Keep new persistence migrations compatible with existing databases; fresh-install data removal is a release preparation step, not a startup migration.

Do not add an AI moderator. Model reliability work belongs in provider adapters, bounded repair prompts and proportionate deterministic validation. Failures should retain logs, recoverable work and user control. Source checks are not a mathematical correctness proof. Model names and capabilities vary by provider; do not download or change a user's model automatically.

## Release and repository status

The included CI checks formatting, Cargo tests/build, launcher tests, UI harnesses and API documentation freshness.

Before publishing, inspect the actual Git staging area for user content and secrets. Default runtime paths are ignored; custom paths need explicit exclusion. Do not assume `.gitignore` scrubs existing history. This checkout may be delivered without writable Git metadata; no commit, push or remote publication is implicit in these files. Choose a license and add its text before distributing under a license; this repository does not invent a copyright owner or grant rights it cannot establish.

Acceptance testing should include your browser, chosen real model/provider, review/import flow, cancellation/restart, notifications, a restored backup, and your deployment's HTTPS/authentication. These complement the automated fake-model and component tests. Frontend CDN availability, real provider output quality, GPU residency and browser visual layout are not proven by unit tests.
