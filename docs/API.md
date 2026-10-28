# API reference

Generated route/handler inventory. Regenerate with `python3 scripts/document-api.py`; CI checks it with `--check`. Paths below include the unified public prefix. GET routes also support HEAD through Axum. Securing access is the user’s responsibility; there is no built-in login. This is a trusted single-user API, with no per-user authorization.

## Conventions

JSON requests use `Content-Type: application/json`. Successful handlers return JSON unless they explicitly return HTML, source bytes or exports. Module errors normally return HTTP 400 with `{"error":"message"}`. StudyHub has no built-in login. Missing routes use 404. IDs are server-generated integers. Multipart uploads have a 64 MiB request limit. The handler and its request type are the authoritative contract for fields and validation.

## Shell

| Method | Path | Result |
| --- | --- | --- |
| GET | `/` | Unified HTML workspace. |
| GET | `/logo.png` | Bundled StudyHub logo (`image/png`). |
| GET | `/api/health` | `service`, `ok`, `unified`, `notes`, `learning`. A responding unified server has both initialized modules. |
| GET | `/notes/`, `/learn/` | Redirect to each module root. |

## Notes

Source: [src/lib.rs](../src/lib.rs). Handlers in child modules are referenced by their module name.

| Methods | Public route | Registration / handler |
| --- | --- | --- |
| GET | `/notes` | `get(&#124;&#124; async { Html(include_str!("../web/index.html")) })` |
| GET | `/notes/api/health` | `get(&#124;&#124; async { Json(json!({"service":"notes","ok":true})) })` |
| GET, PUT | `/notes/api/integration/config` | `get(integration::config_get).put(integration::config_put)` |
| GET | `/notes/api/integration/folders` | `get(integration::folders)` |
| GET | `/notes/api/integration/notes` | `get(integration::list)` |
| GET | `/notes/api/integration/notes/:id` | `get(integration::note)` |
| GET, PUT | `/notes/api/settings` | `get(get_settings).put(put_settings)` |
| GET | `/notes/api/models` | `get(models)` |
| GET | `/notes/api/lookup` | `get(lookup)` |
| POST | `/notes/api/submit` | `post(submit)` |
| GET | `/notes/api/review/sessions` | `get(sessions)` |
| GET, PUT | `/notes/api/review/session/:id/style` | `get(note_preferences::read).put(note_preferences::write)` |
| DELETE | `/notes/api/review/session/:id/logs` | `axum::routing::delete(clear_logs)` |
| GET, DELETE | `/notes/api/review/session/:id` | `get(session).delete(delete_session)` |
| POST | `/notes/api/review/session/:id/redo` | `post(redo)` |
| GET, PUT, POST | `/notes/api/review/session/:id/human` | `get(human_review) .put(save_human_review) .post(complete_human_review)` |
| GET | `/notes/api/review/session/:id/source/:source` | `get(original_source)` |
| GET | `/notes/api/review/session/:id/pdf/:name` | `get(pdf_file)` |
| GET | `/notes/api/review/session/:id/question` | `get(question)` |
| GET | `/notes/api/review/session/:id/page` | `get(page_img)` |
| POST | `/notes/api/review/session/:id/answer` | `post(answer)` |
| GET | `/notes/api/lessons` | `get(lessons_list)` |
| GET | `/notes/api/big_ideas` | `get(big_ideas)` |
| GET | `/notes/api/vault/document` | `get(big_ideas::document)` |
| DELETE | `/notes/api/vault/folder` | `axum::routing::delete(delete_folder)` |
| POST | `/notes/api/review/session/:id/retry` | `post(retry)` |
| POST | `/notes/api/review/session/:id/kill` | `post(kill)` |
| DELETE | `/notes/api/lessons/:id` | `axum::routing::delete(lesson_delete)` |

## Learning

Source: [apps/SelfHosted-AI_Tests/src/lib.rs](../apps/SelfHosted-AI_Tests/src/lib.rs). Handlers in child modules are referenced by their module name.

| Methods | Public route | Registration / handler |
| --- | --- | --- |
| GET | `/learn` | `get(&#124;&#124; async { Html(include_str!("../web/index.html")) })` |
| GET | `/learn/api/health` | `get(&#124;&#124; async { Json(json!({"service":"learning","ok":true})) })` |
| GET, PUT | `/learn/api/settings` | `get(settings_get).put(settings_put)` |
| GET, POST | `/learn/api/providers` | `get(providers_get).post(providers_put)` |
| POST | `/learn/api/providers/:id/test` | `post(provider_test)` |
| GET | `/learn/api/providers/:id/diagnostics` | `get(provider_diagnostics)` |
| POST | `/learn/api/providers/models` | `post(provider_models)` |
| GET | `/learn/api/notes` | `get(notes_list)` |
| GET | `/learn/api/notes/:id` | `get(notes_detail)` |
| GET | `/learn/api/notes/:id/source/:source` | `get(notes_source)` |
| GET | `/learn/api/notes/:id/pdf/:name` | `get(notes_pdf)` |
| GET | `/learn/api/notes/folders` | `get(notes_folders)` |
| GET, PUT | `/learn/api/notes/config` | `get(notes_config_get).put(notes_config_put)` |
| GET, POST | `/learn/api/notebooks` | `get(notebooks_get).post(notebook_create)` |
| GET, PUT, DELETE | `/learn/api/notebooks/:id` | `get(notebook_get) .put(notebook_update) .delete(notebook_archive)` |
| POST | `/learn/api/notebooks/:id/import` | `post(notes_import)` |
| POST | `/learn/api/notebooks/:id/sources` | `post(source_add)` |
| POST | `/learn/api/notebooks/:id/upload` | `post(source_upload)` |
| GET | `/learn/api/sources/:id/file` | `get(source_file)` |
| PUT, DELETE | `/learn/api/sources/:id` | `axum::routing::put(source_update).delete(source_delete)` |
| POST | `/learn/api/notebooks/:id/sync` | `post(notes_sync)` |
| GET, POST | `/learn/api/threads` | `get(study::threads_get).post(study::thread_create)` |
| GET, PUT | `/learn/api/threads/:id` | `get(study::thread_get).put(study::thread_move)` |
| POST | `/learn/api/threads/:id/chat` | `post(study::chat)` |
| POST | `/learn/api/generate` | `post(study::generate)` |
| GET | `/learn/api/jobs` | `get(study::jobs_get)` |
| GET | `/learn/api/runtime/logs` | `get(runtime::get)` |
| POST | `/learn/api/jobs/:id/retry` | `post(study::job_retry)` |
| POST | `/learn/api/jobs/:id/cancel` | `post(study::job_cancel)` |
| GET, PUT | `/learn/api/artifacts/:id` | `get(study::artifact_get).put(study::artifact_edit)` |
| GET | `/learn/api/artifacts/:id/export` | `get(study::artifact_export)` |
| POST, DELETE | `/learn/api/artifacts/:id/share` | `post(study::share_create).delete(study::share_delete)` |
| GET | `/learn/api/shared/:token` | `get(study::shared_get)` |
| POST | `/learn/api/artifacts/:id/attempts` | `post(study::attempt_create)` |
| GET, PUT | `/learn/api/attempts/:id` | `get(study::attempt_get).put(study::attempt_state)` |
| POST | `/learn/api/attempts/:id/answer` | `post(study::answer)` |
| GET, POST | `/learn/api/attempts/:id/questions/:question/journal` | `get(learning::journal_read).post(learning::journal_save)` |
| POST | `/learn/api/attempts/:id/questions/:question/section` | `post(learning::reveal)` |
| POST | `/learn/api/attempts/:id/questions/:question/hint` | `post(learning::hint)` |
| POST | `/learn/api/attempts/:id/questions/:question/step` | `post(learning::step)` |
| GET | `/learn/api/journal` | `get(learning::journal_get)` |
| GET | `/learn/api/notebooks/:id/mastery` | `get(learning::mastery_get)` |
| GET, POST | `/learn/api/threads/:id/corrections` | `get(learning::corrections_get).post(learning::correction_create)` |
| POST | `/learn/api/corrections/:id/apply` | `post(learning::correction_apply)` |
| POST | `/learn/api/attempts/:id/finish` | `post(study::finish)` |
| POST | `/learn/api/attempts/:id/analyze` | `post(study::analyze)` |
| GET | `/learn/api/history` | `get(study::history)` |
| DELETE | `/learn/api/history/:kind/:id` | `axum::routing::delete(history::delete)` |
| GET | `/learn/api/progress` | `get(study::progress)` |

## Common request examples

Examples assume a model profile and an existing notebook. Field defaults and constraints are enforced by the handlers.

```json
// POST /learn/api/providers
{"label":"Local model","kind":"ollama","base_url":"http://127.0.0.1:11434","model":"your-installed-model","context":8192}

// POST /learn/api/notebooks
{"title":"Mathematics","goal":"Understand the methods in my notes","instructions":"Ask one question at a time"}

// POST /learn/api/notebooks/1/import
{"note_ids":[1,2]}

// PUT /learn/api/sources/1
{"teach":true,"quiz":false}

// POST /learn/api/generate
{"kind":"guided_practice","notebook_id":1,"provider_id":1,"prompt":"","count":3}

// POST /learn/api/attempts/1/answer
{"question_id":0,"response":1}
```

These are separate JSON requests; remove the explanatory comments before sending. Generation returns a job identifier; query `/learn/api/jobs` for status/artifact. Cancelling uses POST `/learn/api/jobs/:id/cancel`. Retrying uses POST `/learn/api/jobs/:id/retry` and creates a new job. Live logs are GET `/learn/api/runtime/logs?job_id=…&after=…` or `?thread_id=…&after=…`.

Notes uploads use multipart form fields: see `submit` in the Notes library and the UI upload form for filing metadata. Draft review is GET/PUT/POST `/notes/api/review/session/:id/human`: PUT saves edits; POST explicitly approves or discards the review according to its action. Never assume a generation result is already approved. Bulk multipart uploads may explicitly set `unknown=true` and `skip_human=true` to skip questions/review and save automatically. `skip_human` is ignored for normal or appended uploads. Bulk requests also accept optional `class_type` and `course_name`: supplied values apply to each file and are preserved; blank values are inferred.

History DELETE `/learn/api/history/:kind/:id` accepts material, conversation, session, job, journal and progress kinds. Parent deletion removes dependent records transactionally; running work is protected. Journal IDs identify questions within attempts. Read the deletion handler for body scope and progress reset semantics.

## Request types and data model

Typed request structures include `Generate` in `study.rs`, `Provider` in `providers.rs`, Notes `Settings`, and the scoped query types in the owning handlers. Many update endpoints accept JSON values and validate allowed keys directly; there is no generated OpenAPI schema claiming otherwise.

Notes schema: `initialize_database`, `review_schema`, and `note_preferences::schema`. Learning schema: `schema` in the library, plus `learning::schema` and `runtime::schema`. These functions define table columns, indexes and backward-compatible migrations. Browser state and source originals are documented in [configuration](CONFIGURATION.md). See [architecture](STUDY_SUITE.md) for ownership and history invariants.
