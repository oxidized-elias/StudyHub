# Source-backed study workflows

## Check an explanation or problem

Use **Check Explanation / Sources** under a tutor message, or **Check or Correct This AI Problem** beside a problem. Ask your question or describe the suspected error. You may supply an authoritative public reference URL when the notebook itself may be inaccurate.

The tutor checks the saved source snapshot. A supported explanation identifies the source and exact section. An unsupported check stays in **Needs Source** rather than inventing evidence. When an error is found, a proposal appears under **Source Checks and Corrections**. Review its explanation and source excerpts, then apply it. Material corrections create a new version; conversation corrections append a correction. Original notes, attempts and personal journal entries remain available.

The ordinary tutor also checks citation IDs/section numbers and retries unsupported explanations. It has a scoped `propose_correction` tool to acknowledge and propose fixes to its own earlier work. This checks that cited evidence exists; factual interpretation and open-answer grading still depend on the configured model. Applying a correction to an AI problem excludes the original faulty problem from learner mastery evidence. Historical scores remain the original record.

This workflow corrects learning materials and tutor explanations. It does not silently overwrite the Obsidian source note. If the source itself needs editing, use NoteSorter’s review/edit workflow and refresh the notebook for future sessions.

## Step-by-step practice

Choose **Step-by-Step** on the start screen, or request **Step-by-Step Practice** from the tutor. The model creates an overall problem with small steps. Submit your working for the current step. A correct step unlocks the next; an incorrect step stays open for another attempt. Hints and feedback are saved. Reloading or restarting the server preserves progress.

Future step answers and explanations are withheld by the API. The conversation receives only the current step and your saved working when helping with an unfinished guided problem. Exact expected responses can be accepted directly; other responses use the configured model to judge equivalent reasoning. Failed/invalid grading responses do not mark work wrong.

## Concept mastery and targeted quizzes

Open **History & Progress → Progress**, select a notebook, and choose **Create Targeted Quiz**. Alternatively enable **Target weak concepts and mix in known material** when creating a quiz.

Evidence comes from saved answers, including unfinished sessions and mistakes during unfinished guided practice. Independent correct answers, incorrect answers, assisted responses and explicit skips are separate. Checking a source after submitting an answer does not retroactively turn an independent response into an assisted one. Old questions without concept metadata fall back to their saved section names; generating a fresh quiz gives more specific concept labels.

The current heuristic labels a concept Known after at least two independent correct answers and at least 80% independent accuracy. When both weak and known concepts are available, quizzes use approximately 70% focus and 30% review, with at least one review question when there is room. If only one group has evidence, the quiz uses that group. Disabled notebook quiz sources are filtered from targeting. The model must follow the ordered concept/role schedule or generation retries.

## Problem Journal

Every quiz question, card, guided problem and result review offers **Problem Journal · Add Your Own Note**. Save any problem, even before answering or when the answer was correct. Add your explanation, uncertainty, alternative method, or reminder. Saving again updates your personal note.

Open **History & Progress → Problem Journal** to filter by notebook or show all notebooks. **Open Saved Problem** restores the relevant question or expands its completed result. Journal notes remain attached to the original attempt and problem across AI material revisions.

## Closed-book recall

Choose **Quiz**, enable **Closed-book recall**, and select a notebook. The option defaults to open-ended responses. Answer first, then choose **Reveal Relevant Note Section**. The server returns only the question’s referenced section(s), not the whole note. Citation clicks in this mode use the same section-only path, including after completion.

Headed notes use their actual sections. Sources without headings use bounded paragraph excerpts. Newly generated sourced questions carry precise references; older questions without them need regeneration to use section recall. Whole-note navigation remains available in ordinary learning modes. Practice tests retain their finish-before-explanation rule.

There is no daily review queue or calendar integration in these workflows.

## Implementation and verification

- `apps/SelfHosted-AI_Tests/src/learning.rs`: journal schema/APIs, mastery, focus/review scheduling, section extraction/reveal, guided step grading, correction proposals/application and citation validation.
- `apps/SelfHosted-AI_Tests/src/study.rs`: generation metadata, guided tools, protected answer delivery and tutor context.
- `apps/SelfHosted-AI_Tests/web/index.html`: journal, source checks, guided runner, mastery panel and recall controls.
- `tests/study_workflows.cjs`: UI handlers and workflow controls.

Backend tests cover journal isolation, source bounds, citation validity, recall gating, mastery mixing/assistance, durable guided progression, partial guided errors, and version-preserving corrections. Component tests do not substitute for testing browser layout and real model output. Local socket restrictions prevent a full three-server browser smoke test in this environment.

## Applied problems and keyboard submission

Guided practice now requires concrete givens and 2–12 genuine solving steps. Math problems need numerical expressions or values, rather than the general symbolic definition. Generation and tutor tools reject generic symbolic givens without concrete data, copied source examples and repeated previous prompts; invalid generations retry with the validation reason. Wording and optional quotations are not blocking requirements. Recent notebook prompts are supplied to generation so it can vary tasks. Sources support the method; the new problem data is generated practice. These structural/duplicate checks do not constitute a formal mathematical proof of model answer correctness.

Older definition-only guided exercises display **Create Applied Problems** to generate a replacement while preserving history. Givens are displayed separately so the learner always sees the numbers/scenario being used.

Enter submits chat, guided responses, open quiz responses, start prompts, correction concerns, journal notes and NoteSorter AI guidance. Shift+Enter inserts a newline. IME composition and repeated held keys do not submit. Full note and study-material editors remain document editors.

## Source-limited, question-first tutoring

Tutor chat requires selected notebook sources. Help with a supplied problem stays in chat; artifact creation requires an explicit practice/create request. Ordinary tutor replies have a maximum of 85 words and exactly one question, with bounded retries using the configured provider. Exhausted retries return a short request to choose a note section instead of displaying an unsupported draft.

Generated guided problems cite supporting note sections; verbatim quotations are optional. Unverified optional quotes are removed rather than blocking the exercise. Step grading receives actual section text from the source snapshot. Runtime guards also reject several named methods absent from the source snapshot, including the discriminant, quadratic formula, completing the square and complex roots. Old unsupported exercises show a replacement action; their history remains saved. These checks catch missing evidence and known method mismatches; they are not a complete semantic proof that every model claim follows from a note. Source quality and real-model evaluation remain relevant.

Local model research: [[LOCAL_TUTOR_MODELS]].

## Delete saved history

History offers dark-red Delete buttons for materials, practice sessions, conversations, completed/failed generation jobs, and individual problem journal entries. Each opens a confirmation describing the scope before calling `DELETE /api/history/:kind/:id`.

- Material deletion removes its attempts, journal entries and corrections, revokes its shared link, and detaches generation-job links. Other material revisions and the conversation remain.
- Conversation deletion removes its messages, materials and their dependent records.
- Session deletion removes its answers/working and journal entries. Scores and mastery are recalculated from remaining sessions.
- Journal deletion removes only that saved problem entry and personal note. Job deletion removes only the job record.
- Clear Progress resets all sessions, or only the selected notebook's sessions, including their journal entries. Source notes and notebooks remain.

Foreign-key cleanup is transactional. Active study requests and relevant running generation jobs must finish before deletion. No existing user data is deleted when this feature is installed.

Verification: `cargo test --workspace --offline`; `TYPESCRIPT_RUNTIME=/opt/vscodium-bin/resources/app/extensions/node_modules/typescript/lib/typescript.js node tests/history_delete.cjs`.

## Natural problem wording and model selection

Guided-practice validation accepts natural problem prompts such as “What are the degree and leading term…?”, “Sketch…”, and “Identify…”. It no longer requires a verb from an English whitelist. Concrete givens, numerical math inputs, multiple steps, source evidence, and copied/reused-example checks still apply. Failed generation retries receive a bounded copy of the rejected draft alongside the precise validation error.

Settings uses dropdowns for model IDs, preserving the configured model even when discovery fails. Learn discovers installed models from local/remote Ollama (`/api/tags`) and compatible servers (`/models`) through `POST /api/providers/models`. A saved credential is reused only for the same endpoint and provider. Cloud options include saved models and a custom-ID escape hatch; cloud discovery is not implemented. Notes settings discovers installed Ollama models without running the optional recommendation tool. Users can explicitly enter custom IDs in either settings UI.

Regression checks: `tests/model_select.cjs`, the provider discovery parser test, and the natural-wording cases in `guided_generation_rejects_definition_only_and_reused_problems`.

## Warning dismissal and live diagnostics

Warning/error alerts have accessible close buttons alongside existing actions. Dismissing an unchanged failed-job banner is saved in this browser; the job remains in History. A changed failure or a different job appears again.

Session logs are collapsed by default. Expand **Live Session Logs** beneath an active generation job or in a study conversation. Saved job logs also appear under Generation Jobs in History. Open panels poll `GET /api/runtime/logs` every 1.5 seconds using a scoped job/conversation ID and incremental cursor; closed panels stop polling. Logs retain stage timestamps, provider/model, configured context, request timing, HTTP status, response size, Ollama timing/token metrics, validation retries, and final status. Keys, full prompts and full responses are excluded. Provider error messages are limited and redact the configured secret; arbitrary non-JSON error bodies are omitted.

**Check Model Status** uses the saved profile to inspect Ollama version, installed models and loaded models. It shows runtime GPU memory and actual/configured context; it does not load, pull or unload a model. Other providers continue to expose request stages and HTTP errors. Application logs cannot reconstruct old requests or replace the Ollama host's server logs.

Small-model generation uses a concise mode-specific prompt, schemas using actual available IDs, and an explicit allowed-reference table. A problem's topic label and its citation section number are distinguished. Guided problems are generated one per request with 2–6 requested steps. Reference normalization accepts integer strings, explicit `[S#.section]` citations, or unique exact section headings. A missing section may be inferred only from a unique exact supporting quote or a source containing only one section. Unknown IDs, nonexistent sections and ambiguous evidence remain errors. These checks validate reference locations; they do not prove every model claim is supported.

### GPU placement

Ollama's CPU/GPU split describes model memory placement, not compute utilization. Larger context windows consume more memory. Settings offers 4K/8K/16K presets; changing a preset edits the profile, and Save Profile applies it to subsequent requests. The application leaves GPU placement to Ollama and does not claim that a chosen preset guarantees full GPU residency. Check available VRAM, other loaded models and the model's quantization on the Ollama host. See [Ollama FAQ](https://docs.ollama.com/faq) and [context memory guidance](https://docs.ollama.com/context-length).

Verification includes `tests/warning_dismiss.cjs`, `tests/live_logs.cjs`, runtime-log scoping/redaction tests, and reference-normalization regression tests.


## Convenient generation and cancellation

No moderator/reviewer model runs in this workflow. The configured model generates content; ordinary Rust checks validate structure and source boundaries. The former exact `method_quote` gate was removed because small models paraphrase legitimate source material. Optional exact quotes can remain; unverified optional quotes are dropped, while grading reads the actual referenced note sections. Definitions are rejected based on absent concrete givens, not a wording blacklist.

If later batches fail after valid questions were generated, the completed questions are saved as usable material with an informational partial-result notice. Invalid questions are not published. The saved artifact records `requested_count` and `generation_warning`.

**Kill Task** beside the top active-job progress bar posts `/api/jobs/:id/cancel`. A durable `cancelled` state causes the worker to drop its in-flight generation/HTTP future. Atomic publication checks prevent a cancelled job from publishing a late result. Saved notes/materials remain. A finished task keeps its completed result if cancellation arrives too late. Retry creates a new job ID so the old cancelled worker cannot resume. Cancellation ends the application's request; the remote provider controls its own worker cleanup.

## Formatted user input

Message, reasoning, guidance and journal text fields use a rich Markdown input control. Powers such as `2^3` render with superscripts while typing; bold, emphasis, inline code and explicit dollar-delimited math also render. Original Markdown is retained for submission, source-preserving copy/paste and undo/redo. Code fences remain literal. Enter/Shift+Enter and IME composition behavior are retained. Long JSON/source-document editors remain plain editors.

`tests/task_controls.cjs` checks task cancellation UI, collapsed logs, rendering/source retention and escaping. `tests/rich_input.cjs` exercises native Chromium caret/typing/undo/paste behavior using a local file; it cannot run under this environment's blocked Chromium socket operations. Browser testing and real model calls remain unverified here.

## Notification popup stability

StudyHub Snackbar keeps a native `div` directly inside its Grow transition. The dismissible `Notice` function does not forward a DOM ref and can return null when closed, so it must not be the transition’s direct child. A missing transition ref can crash the React shell and expose the black page background. Native desktop notification constructors run outside state updaters and are individually guarded. Notification persistence tolerates unavailable storage; invalid saved inbox entries are filtered. In-app popups and the inbox remain usable when desktop notifications fail. `tests/notifications.cjs` checks the DOM-child contract, open/close, deduplication, frame preservation, native-notification failures, and storage failures.
