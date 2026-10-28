# Configuration and operations

## Runtime

`node server.mjs` builds the integrated `study-hub` executable and runs it with inherited environment variables. There is one server process beneath the Node launcher, one address, and no child backend servers. Node forwards SIGINT/SIGTERM and reports nonzero exits. The native server handles both signals with graceful HTTP shutdown. Active generation interrupted by shutdown becomes retryable on the next startup; there is no automatic replay of paid requests.

| Variable | Default | Purpose |
| --- | --- | --- |
| `STUDYHUB_BIND` | `127.0.0.1:8082` | The single HTTP listener. |
| `HUB_BIND` | unset | Compatibility fallback if `STUDYHUB_BIND` is unset. |
| `STUDYHUB_URL` | `http://127.0.0.1:8082` | Public origin for application links. Set when changing ports or using HTTPS. |
| `DATA_DIR` | `<repository>/data` | Notes settings, sessions, uploads and correction lessons. |
| `LEARNING_DATA_DIR` | `<DATA_DIR>/learning` | Notebook/history database and learning source originals. |
| `VAULT_DIR` | `<DATA_DIR>/vault` | Initial vault path. Saved UI settings take precedence after configuration. |
| `CARGO_TARGET_DIR` | `<repository>/target` | Build and executable directory. |
| `STUDYHUB_USER` | `admin` | HTTP Basic username when a password is enabled. |
| `STUDYHUB_PASSWORD` | unset | Enables authentication on the shell, both modules and their APIs. |
| `NOTES_INTEGRATION_TOKEN` | unset | Optional compatibility export token. Internal calls supply it automatically. |

Relative storage paths passed to the Node launcher resolve against the repository, not the current shell directory. The native executable can also be started directly; its relative paths follow its working directory.

The old `BIND`, `LEARNING_BIND`, `NOTES_URL`, `LEARNING_URL`, `NOTES_BIN`, `LEARNING_BIN`, `SUITE_ROOT`, `LEARNING_PROJECT`, `HUB_PROJECT`, and `--connect` options are no longer part of startup. Notes integration runs in memory and does not need an endpoint or token field in Settings. Existing source module directory names are retained for navigation, not separate deployment.

## Models

Configure profiles in AI Models & Settings. The form opens the saved active profile; **Save & Use Model** persists it and the active selection together. Existing profiles are edited in place, and the selection is restored after reload. The notes importer retains its separate author/vision settings. Supported adapters include Ollama, OpenAI, Claude, Gemini, DeepSeek and compatible chat endpoints. The model dropdown discovers installed Ollama or compatible-server models, with a custom identifier fallback. Cloud profiles use a saved/custom model identifier. Remote Ollama endpoints are supported. The notes importer has its own author/vision model selection because transcription may need a vision model.

Provider keys and notes settings reside in server files/databases. Do not commit or publicly serve these directories. The learning profile API redacts keys. Notes settings are available to the authenticated workspace owner. This application has no separate roles or per-user access control.

Context length affects VRAM consumption; reducing it can improve GPU residency. The application does not force Ollama placement or unload models. Check Model Status and Live Session Logs for installation/load state, timing, retry reasons and provider errors. Logs exclude full prompts, responses and credentials; notes' own AI logs contain note content and are private user data.

## Build and deploy

```bash
node server.mjs --release --build-only
node server.mjs --release --no-build
```

Run under a service manager with a non-root account and writable data directories. Set environment variables in that manager or use Node's explicit `--env-file`. Route `/` and every subpath to the single listener. Allow long model requests (up to 20 minutes at the application/provider level) and at least 64 MiB upload bodies. There are no backend ports to expose.

For container deployment, see [Docker setup](DOCKER.md). Compose keeps data at `/app/data` and the vault at `/vault`, with persistent mounts for both. Settings paths then refer to the container filesystem.

## Files and backups

| Path | Contents |
| --- | --- |
| `data/config.json` | Notes settings and notes-provider keys. |
| `data/review/sessions.db` | Import sessions, events, drafts, human reviews, learned metadata and note locations. |
| `data/review/…` | Original files and intermediate processing files. |
| `data/lessons/…` | Human-approved corrections for future notes. |
| `data/learning/learning.db` | Profiles, notebooks, source snapshots, threads, messages, materials, attempts, jobs, journals, corrections, mastery evidence and runtime logs. |
| `data/learning/sources/<id>/original` | Uploaded source bytes. |
| `data/vault/…` or configured vault | Approved Obsidian notes and `BigIdeas.md` indexes. |
| `data/.browser-epoch` | Identifies the data installation for clearing stale browser preferences after reset. |

Stop the application before copying the entire data directory and any external vault/learning directory. If backing up a running SQLite database, use SQLite's backup API and include a consistent snapshot of originals; copying just the main file while WAL is active can lose data. Restore paths and file permissions as well as databases. Vault changes affect future writes; existing notes resolve their recorded original locations and are not automatically moved.

## Deletion and Git hygiene

History offers scoped deletion with confirmations. Clear AI Log acts on one note. Archiving a notebook preserves history. Deleting history does not necessarily delete approved vault Markdown.

For a complete reset, **stop the program**, then remove the configured data directory, separately configured learning directory, and any vault content you intend to delete. This is irreversible; keep backups only if wanted. Next startup creates empty databases. When the unified UI opens with a new data epoch, it clears only app-owned `hub.*`, `study.*`, and `dismissed-warning.*` local/session storage keys, including notifications, selected notebook/job and tutorial progress. Other websites' data is unaffected. Blocked browser storage is tolerated. Remote provider logs, Obsidian trash, filesystem snapshots and external backups are outside this program's storage.

The repository `.gitignore` excludes default data/vault directories, SQLite/WAL files, `.env`, keys, logs, build artifacts, dependencies and local tool metadata. Custom data paths should live outside the checkout or have an explicit ignore rule. Ignoring a file does not remove it from an existing Git history. Inspect staged files before publishing. No existing Git metadata or remote is assumed by this project.

## Troubleshooting

- Missing Cargo or compiler: install the Rust toolchain and native build prerequisites, or run an already-built binary with `--no-build`.
- Offline dependency error: build once with internet access, then use `--offline` with the same Cargo cache.
- Address already in use: stop an older suite instance, or change `STUDYHUB_BIND` and `STUDYHUB_URL`.
- Missing PDF tools: install Poppler and ensure both commands are on PATH.
- Imported note fails: verify the approved Markdown file still exists at its recorded location. The upstream error is included. Restore it or use saved notebook snapshots by disabling refresh.
- Validation retries: inspect Live Session Logs; use a model capable of structured output and selected notes with enough applicable content. The application uses one model and deterministic validation, with no moderator model. Valid earlier batches survive later failures.
- Blank UI: check browser console/network for blocked CDN assets and the server terminal for errors. Notification popups have a stable DOM transition child to avoid the prior MUI transition crash.
