# Historical request audit

This records earlier implementation stages. Current startup, architecture and test commands are documented in [README](../README.md), [architecture](STUDY_SUITE.md), and [development](DEVELOPMENT.md). The former three-server setup has been replaced by one integrated server launched through Node. Counts below describe their historical verification stage.

# Request audit — 2026-10-26

This checklist records the implementation and the verification limits after reviewing the conversation. Source is staged under this writable NoteSorter workspace. It does not claim that a browser session, a real cloud model, or the user's existing Polynomials note has been exercised here.

## Notes and review

| Request | Current behavior and source |
| --- | --- |
| Remove the moderator | Generation uses the author and deterministic validation; obsolete moderator settings are normalized out. `src/lib.rs`. |
| Bulk unknown uploads | Browser queue processes files with recovery and human review. Upload size/concurrency remain bounded; “massive” does not mean an unlimited single HTTP request. `web/index.html`. |
| Optional unit number; infer name | Unit number is optional; author output supplies the unit name. Backend test covers writing a note without a unit number. |
| Human review before finalization | Durable review pauses generation until approval. Original upload/transcription is on the left; editable AI Notes and preview are on the right. `src/review.rs`. |
| Learn from edits and show changes | Review preserves the AI draft, human revisions and feedback; approval produces a lesson. Your Changes shows the diff. |
| Clear prompts / AI log | Note-scoped Clear AI Log uses a dark red standard button. Backend test checks another note in the same subject is retained. |
| Polynomials stuck / requires stopping | Interrupted sessions are retryable; no moderator kill requirement. The actual user note needs runtime checking on the user's server. |
| Markdown, math and Mermaid | Shared parsing improvements support Obsidian callouts, links, math and fenced Mermaid. Invalid diagrams retain readable source. Parser tests run; browser Mermaid layout is not visually verified here. |
| Central Obsidian topic files | Root and per-course `BigIdeas.md` link to real note files. Automatic relationship Mermaid is removed from newly generated notes. Indexes rebuild on save and note deletion. Existing handwritten diagrams are preserved. |
| Three Note Styles | Bullet Points, Text Chunks, Combined. Global Settings and persistent per-note override; override applies on regeneration/additional pages and does not rewrite approved text immediately. `src/note_preferences.rs`. |
| Main system vault | Settings accepts an absolute server path; Learn Settings browses folders on the machine running NoteSorter. Existing files are not moved by changing the setting. |
| Application map | `docs/Application Map.md` and `docs/STUDY_SUITE.md` describe the components, storage and integration paths. |

## Learning and unified UI

| Request | Current behavior and source |
| --- | --- |
| Separate learning app + unified app/server command | Superseded by the unified server: `node server.mjs --offline` builds and starts one executable with Notes and Learn linked as libraries. |
| Gemini-style learning features | Guided learning, quizzes, flashcards, guides, practice tests, notebook source scopes, scoring, hints, reflection, sharing, revisions and history. Detailed feature mapping: `apps/SelfHosted-AI_Tests/FEATURES.md`. |
| Cloud and remote local models | Provider adapters for OpenAI, Claude, Gemini, compatible endpoints and Ollama at a configurable host. Protocol/unit tests run; no real credential/model call was made in this audit. |
| AI-created interactive UI | Scoped tool calls create/revise sandboxed HTML/CSS/JS study canvases with persisted learner state. No unrestricted application-code execution. |
| History / progress | Persistent SQLite records include conversations, source snapshots, artifacts, attempts, answers, results and generation jobs. Every history tab has an empty state. |
| Notebook folders and files | Expandable selection tree imports individual notes or a whole folder. Folder uploads allow selecting files before transfer. Note preview compares upload/transcription with AI Notes. |
| Interactive tutorial | Nonmodal walkthrough highlights real controls and waits for user actions. Covers models, vault, note approval, notebooks, source roles, modes, history and notifications. Explicit Skip Step and minimize/close are available. Completion removes the toolbar entry; replay remains in Settings. |
| Notifications | StudyHub stores an inbox in this browser and polls for processing completion/review/failure. Browser alerts are optional and require permission. This is not a push service when the application is closed. |
| Less rounded controls and aligned checkboxes | Explicit small pixel radii for text fields and review panels; controlled radio labels and source-role columns align their controls. Material UI remains in use. Browser pixel/layout checks are still outstanding. |
| Quiz answer submission | Radio selection works. Submitted quiz answers lock and the button is disabled; unchanged practice-test responses also disable resubmission. |
| Quiz tutor timing | Tutor input becomes available after the current quiz question has a non-skipped submitted answer. Backend enforces the gate. Practice tests hold explanations until the test is finished. |
| Answer eliminator | Multiple-choice quiz/practice answers can be eliminated/restored. Eliminating the selected answer clears selection; elimination state survives reload. Server rejects submitting an eliminated answer. |
| Clickable source citations | Chat, card and result citations use the same handler. A NoteSorter source opens the rendered note and highlights its cited heading/section. Other sources open a highlighted saved snapshot. Code-fence headings do not change section numbering. |
| Consistent mode names | User-facing material kinds use Quiz, Practice Test, Study Guide, Guided Learning, Flashcards and Interactive Tool instead of backend enum strings. |
| Speed | Workspace state updates replace the previous repeating attempt poll. History/progress load when relevant, source files load on demand, and frames stay mounted. No measured production performance claim is made. |

## Verification performed

- `cargo test --workspace --offline`: 32 tests passed (13 Notes, 17 Learn, 2 StudyHub).
- `cargo build --workspace --offline`: all three applications build.
- `tests/ui.cjs`: bulk queue, exact review submission and note-scoped log action.
- `tests/suite_ui.cjs`: JSX compilation, proxy prefixes, trusted iframe handoff, canvas isolation/state bridge, folder/file selection and history empty states.
- `tests/request_audit.cjs`: actual handlers for radio selection, elimination and selected-answer clearing, submit locking, tutor gating, matching citation ordinals, action-driven tutorial and source preview.
- `tests/render.cjs`: real Markdown/KaTeX parsing with cached test runtimes. This is not a browser run of the production CDN versions.

## Environment limits and remaining runtime checks

The application is self-contained in this repository. Sibling installation and the old multi-process launcher have been removed.

Local socket creation/binding fails with `Operation not permitted`. Consequently the three-server smoke test and real browser click/layout checks could not run here. `tests/suite_smoke.py` is available for a normal host. Verify the actual existing Polynomials note, original PDF display, section scrolling, desktop/mobile layout, remote model connections and completion notifications in that running environment.

## Follow-up: reviewed import HTTP 400

The configured vault had changed while Polynomial Functions still existed in the original repository vault. Imports previously read only the new path and hid the missing-file reason. Approved notes now retain an absolute location in `note_locations`; existing notes can recover from the original repository vault, and changing Settings records existing locations before switching. Missing-file errors explain how to recover. Learn preserves the upstream error detail. No note files are moved. The follow-up suite passes 34 Rust tests (14 Notes, 18 Learn, 2 Hub), including importing an approved note after a vault switch and preserving integration error details.

## Follow-up: formatting, notes navigation, chat and practice

- Number controls share Material UI minus/plus buttons, a consistent outline and numeric input validation. The practice format and timer have dedicated width and use a stacked layout on narrow screens.
- Rendering repairs LaTeX control characters produced by malformed JSON escapes, including existing saved cards/guides. The repair is scoped to math and preserves code literals. Grouped source references become separate clickable section links.
- Future flashcards target terminology, definitions, symbols and concise examples. Validation rejects multiple-choice-style decks. Existing decks have Create Terminology Deck; old content is retained until the learner chooses to replace it.
- Notes have subject/course disclosure controls, title/subject/course search, and a collapsible Add Your Notes panel. Collapsing retains upload state.
- StudyHub adds Back to Learning after a reference opens Notes; it returns to the mounted learning frame without resetting the conversation. Standalone Learn embeds the referenced note behind its own Back to Learning control.
- Enter sends chat; Shift+Enter inserts a newline, and composition input is respected. User messages are optimistic and progress appears underneath them. Failed sends restore the draft and fetch server history.
- User message padding/radius are reduced. Mode names and conversation headings use readable capitalization.
- StudyHub completion updates also show an in-app popup, with Open and Close actions, in addition to the persisted notification inbox and optional browser notifications.
- The saved canvas examined during this fix had a JavaScript syntax error and simulated persistence. Sandbox runtime errors now appear in the parent with Repair with Tutor and native practice-problem actions. New canvas validation rejects replacing the real Study API. AI-generated code still requires runtime validation; this change does not claim every generated program is correct.
- The tutor has Generate Practice Problems and Connect My Notes actions. The existing scoped tools create native quizzes with explanations and source citations; canvases are reserved for genuine interactive visualizations/manipulatives.

Verification: 35 Rust tests pass (14 Notes, 19 Learn, 2 Hub), all three applications build, and six frontend/parser scripts pass. New checks cover optimistic chat ordering, Enter/Shift+Enter, numeric controls, searchable/collapsible note branches, reference return, malformed saved math, grouped citations, and sandbox error reporting. These are component/parser tests; real browser layout, real model output quality, and server notification delivery still require a host that permits local listeners. Existing saved decks/canvases are preserved; rebuilding/repairing them uses the user's configured model.

## Follow-up: focused study features

Implemented source-backed explanation checks and correction proposals, durable step-by-step practice, concept evidence with targeted weak/known quizzes, an all-problem journal with personal notes, and closed-book section-only recall. The daily queue is explicitly omitted. Usage and limits are documented in `docs/LEARNING_WORKFLOWS.md`.

Verification now includes 42 Rust tests (14 Notes, 26 Learn, 2 Hub) and the new `tests/study_workflows.cjs` component checks. These verify persistence, source bounds, hidden guided answers, mixed concept scheduling, correction versioning and exclusion from learner mistake evidence. Full browser/server and real-provider checks remain subject to the environment limits above.

## Follow-up: applied guided practice and Enter submission

Added a shared keyboard-submit handler across response/message inputs, including previously missing quiz answers and the start prompt. New checks verify Shift+Enter, IME composition and held-key behavior. Guided generation now requires concrete givens, numerical math inputs and at least two reasoning steps; duplicate prompts, copied source givens and definition-only questions are rejected. Existing recall-style exercises have Create Applied Problems without deleting their history.

Verification: 43 Rust tests and eight frontend/parser scripts, including `tests/keyboard.cjs`. Real model quality and browser layout still require runtime verification on the user's host.
