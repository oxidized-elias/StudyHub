# Repository Guidelines

## Project Structure & Module Organization

StudyHub is a Cargo workspace with a Node.js launcher. `src/` implements the `notes-web` library; `apps/SelfHosted-AI_Tests/src/` implements learning features; `apps/StudyHub/src/main.rs` integrates both into the `study-hub` executable. Only the integrated executable binds a socket. Frontend HTML/JSX lives in `web/` and each application's `web/` directory. `tests/` contains launcher, UI, Docker, and integration checks; Rust unit tests live alongside their modules. `scripts/` contains development utilities, `docs/` contains reference documentation, and `logo.png` is the project logo.

## Build, Test, and Development Commands

Use Node.js 22+, stable Rust/Cargo, and native C build tools. PDF imports require Poppler. Run commands from the repository root:

- `npm start`: build and start the debug application on port 8082.
- `npm run build`: build the optimized executable without starting it.
- `npm test`: run Node launcher tests and all workspace Rust tests with the committed lockfile.
- `cargo fmt --all -- --check`: verify Rust formatting.
- `npm run test:ui`: run UI/parser harnesses; install the optional runtimes documented in `docs/DEVELOPMENT.md` first.
- `npm run test:docker-config`: validate container configuration and health checks.
- `python3 scripts/document-api.py --check`: check API documentation freshness.

## Coding Style & Naming Conventions

Use Rust 2021 conventions and rustfmt, with four-space indentation, `snake_case` functions/modules, and `PascalCase` types. Follow existing two-space indentation and naming in JavaScript files. Use `.mjs` for Node ES modules and `.cjs` for CommonJS test harnesses. Keep routes in their owning library; learning features must call the Notes router instead of querying its database directly.

## Testing Guidelines

Use Rust's built-in test framework and Node's `node:test` for launcher checks. Name regression tests after the behavior they protect and follow existing `*.test.mjs` or feature-specific `.cjs` patterns. Run relevant UI checks for frontend changes. Python smoke tests start temporary servers and fake Ollama endpoints. No numeric coverage threshold is documented.

## Commit & Pull Request Guidelines

Git history is unavailable in this checkout. Use concise, imperative commit subjects. PRs should describe the changed behavior, link relevant issues, report validation, and include screenshots for visible UI changes. Regenerate `docs/API.md` after route changes and update the application map when moving modules.

## Configuration & Persistence

Use `.env.example` and `.env.docker.example` as configuration references. Exclude provider keys, uploads, databases, and vault content from commits. Keep migrations compatible with existing databases; never reset user data during startup.
