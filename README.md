<p align="center">
  <img src="logo.png" width="120" alt="StudyHub logo" />
</p>
<h1 align="center">StudyHub</h1>
<p align="center">
  Self-hosted notes, notebooks and AI-assisted studying in one Material UI application.<br>
  Import and review notes, save them to an Obsidian vault, and use selected sources for guided learning, quizzes, flashcards, study guides and interactive activities.<br>
  History, corrections, journals and progress are saved locally.
</p>

<br>


## Start

Requires **Node.js 22+**, a current stable **Rust toolchain/Cargo**, and a C compiler for bundled SQLite. Install **Poppler** (`pdftoppm`, `pdftotext`) for PDFs. An AI provider is configured separately; local models run on any reachable Ollama server.

```bash
node server.mjs
```

Open **http://127.0.0.1:8082**. This builds and starts **one integrated server**, not three applications. Node manages its lifetime; notes and learning share one native executable and one listener. Rust is retained for the existing processing engine. No npm dependencies are required to run it. `npm start` and `./start.sh` are aliases.

For an optimized build:

```bash
node server.mjs --release
```

The first build downloads Cargo dependencies. Use `--offline` when they are cached. `--build-only` builds without starting; `--no-build` starts an existing binary (use the same `--release` setting). Run from any working directory. Ctrl+C stops the application.

## First use

1. In **AI Models & Settings**, add a local or cloud model profile and test it. Configure the notes author/vision model in Notes settings when importing images.
2. Select an Obsidian vault folder on the host, and a default Note Style: bullet points, text chunks, or combined. Per-note overrides are available.
3. Import notes, review the source and AI draft side by side, edit and approve. Approval saves the final Markdown and correction lessons. Bulk uploads additionally offer an opt-in Skip human intervention switch for unattended automatic saves.
4. Create a notebook. Import individual notes or expand the subject/course tree and select folders/files. Choose Learn and Quiz sources independently.
5. Start a study mode, or ask the tutor about your selected notes. Guided practice reveals one step at a time. History retains attempts, conversations and generated material.

The interactive tutorial guides these actions; replay it from Settings. See the [user guide](docs/USER_GUIDE.md) for the complete workflow.

## Configuration and data

Defaults are `data/` for settings, SQLite history, uploads and learned corrections, and `data/vault/` for approved Markdown. A fresh checkout contains **no user records, keys or saved notes**. These directories are created on first startup and excluded from Git.

```bash
cp .env.example .env
node --env-file=.env server.mjs --release
```

Node only loads `.env` when explicitly requested. Provider keys are entered in the UI and stored in local server data. Frontend libraries currently load from pinned CDNs, so the browser needs internet access even with a local model.

This is a **single-user, trusted workspace**, not a multi-tenant service. Optional HTTP Basic authentication protects all application routes. Use an HTTPS reverse proxy for access beyond your host. Configure `STUDYHUB_BIND`, `STUDYHUB_PASSWORD` and the public URL as described in [configuration and operations](docs/CONFIGURATION.md).

## Documentation

- [Application map](docs/Application%20Map.md): Obsidian-compatible entry point for future contributors/models.
- [Architecture and data flow](docs/STUDY_SUITE.md): modules, persistence, UI bridge and generated-tool contract.
- [User guide](docs/USER_GUIDE.md): imports, review, notebooks, study modes, corrections and history.
- [Configuration and operations](docs/CONFIGURATION.md): environment, deployment, backup, deletion and troubleshooting.
- [API reference](docs/API.md): every registered route and handler, examples and response conventions.
- [Development](docs/DEVELOPMENT.md): source layout, tests, release checks and limitations.
- [Learning workflows](docs/LEARNING_WORKFLOWS.md): detailed grading, mastery, guided practice and correction behavior.
- [Feature coverage](apps/SelfHosted-AI_Tests/FEATURES.md) and [request audit](docs/REQUEST_AUDIT.md): implementation history and scope.

## Checks

```bash
npm test
cargo fmt --all -- --check
node server.mjs --release --build-only
```

UI regression checks need development runtimes only:

```bash
npm install --no-save --package-lock=false typescript@5.8.3 marked@12.0.2 katex@0.16.11
npm run test:ui
```

Alternatively supply existing modules with `TYPESCRIPT_RUNTIME`, `MARKED_RUNTIME`, and `KATEX_RUNTIME`. After a debug build, `python3 tests/suite_smoke.py` tests complete workflows with temporary data and a fake model; it requires local socket access. See [development](docs/DEVELOPMENT.md) for verification boundaries.
