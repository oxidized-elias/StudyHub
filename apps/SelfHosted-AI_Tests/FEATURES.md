# Feature coverage

Reference: [Gemini study-material guide](https://support.google.com/gemini/answer/16275879?hl=en). This implementation supplies self-hosted counterparts, rather than Google account, Chrome companion, or Google Docs integrations.

| Capability | Implementation |
| --- | --- |
| Generate study materials from a prompt | Composer and validated background generation jobs. |
| Supply source context | Reviewed NoteSorter notes, documents, pasted text, public webpages, notebook source-role selection. Browser-page context is supplied by URL or pasted text. |
| Right-side document/material workspace | Persistent conversation beside a Material UI material panel, with full-screen option. |
| Practice exam formats | Configurable SAT/JEE Main/NEET UG templates, up to 200 batched questions, custom sections and timing. |
| Full or section practice | Section chips filter the server-owned question set for an attempt. |
| Countdown and summary | Server-persisted deadlines; per-section score, correct/incorrect/skipped review. |
| Hints and topic questions | Question hints and source-grounded conversation aware of the current question/card. |
| Navigate and continue practice | Previous/next, saved responses, more-question generation without overwriting prior artifacts. |
| Performance reflection | Immediate strengths/focus summary, detailed model analysis, new guided learning from focus areas. |
| Flashcard controls | Hints, flip, previous/next, shuffle, browser speech, restart, recall marking, missed-card sessions. |
| Edit materials | Card forms, editable Markdown guides, versioned canvas code; prior attempts retain their original version. |
| Derive practice from a document | Generate quizzes/cards from the revised guide as an additional source snapshot. |
| Share | Copyable, revocable workspace links; native OS sharing where the browser supports it. |
| Add study conversations to notebooks | Conversation transfer also moves its artifact associations, preserving source snapshots. |
| Notebook organization | Named goals/instructions, source import/edit/remove, role selection, refresh and archive. |
| Guided lessons | Diagnosis, small steps, one question at a time, examples, feedback and notebook-specific prior results. |
| Model choice | OpenAI, Claude, Gemini, compatible endpoints and local/remote Ollama profiles. |
| Agent-created learning interfaces | Scoped tools create/revise real interactive canvases with persisted learner state. |
| History | SQLite conversations, source snapshots, tool receipts, artifacts/revisions, attempts, answers, scores, analysis and jobs. |

Teaching/UI inspiration: [amosblomqvist/learn](https://github.com/amosblomqvist/learn), especially diagnosis-first teaching, graded practice, concise visual explanations, and tools that ask the learner to participate. This implementation uses its own web application and bounded tool dispatcher; it does not copy that project's pi configuration, execute its agents, or require its subagent extensions.

Additional reference: [Guided learning](https://support.google.com/gemini/answer/16448384?hl=en), [notebook organization](https://support.google.com/gemini/answer/16972047?hl=en).

Practical boundaries: model-generated practice is not official exam material. Browser speech depends on device/browser support. Sharing is within the deployment's workspace access model, not a multi-user permission system. Scanned pages go through NoteSorter; uploaded text PDFs use Poppler. Frontend CDN assets and model endpoints require connectivity unless separately cached/self-hosted.
