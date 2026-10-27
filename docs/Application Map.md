---
title: StudyHub Application Map
---
# StudyHub Application Map

This is the Obsidian-compatible starting point for future models and maintainers. Documentation lives outside the user vault so clearing notes cannot erase the application map. Open the repository or `docs/` in Obsidian, or follow the relative Markdown links in Git.

## Big ideas

- [How to run and configure the program](../README.md)
- [Architecture, data ownership and processing](STUDY_SUITE.md)
- [Using imports, human review, notebooks and study modes](USER_GUIDE.md)
- [Environment, deployment, backup and deletion](CONFIGURATION.md)
- [Docker packaging and persistent mounts](DOCKER.md)
- [API route and request-type reference](API.md)
- [Source layout, testing and release process](DEVELOPMENT.md)
- [Guided practice, corrections, mastery and recall](LEARNING_WORKFLOWS.md)
- [Local tutoring model research and limitations](LOCAL_TUTOR_MODELS.md)
- [Feature coverage](../apps/SelfHosted-AI_Tests/FEATURES.md)

## Change navigation

| Change | Read first |
| --- | --- |
| Startup/flags/data paths | [server.mjs](../server.mjs) |
| Authentication/routing/shell HTML | [server entry](../apps/StudyHub/src/main.rs) |
| Notifications/unified navigation | [shell frontend](../apps/StudyHub/web/index.html) |
| Notes author, transcription, upload, recovery | [notes library](../src/lib.rs) |
| Review/edit differences/original previews | [human review](../src/review.rs) |
| Subject big-idea pages/indexes/backlinks | [subject concepts](../src/big_ideas.rs) |
| Vault links/host folders/export | [notes integration](../src/integration.rs) |
| Per-note formatting preference | [note preferences](../src/note_preferences.rs) |
| Notebook/source import/model profiles | [learning library](../apps/SelfHosted-AI_Tests/src/lib.rs) |
| Protocol/model request errors | [provider adapters](../apps/SelfHosted-AI_Tests/src/providers.rs) |
| Chat/tool loop/generation/quiz attempts | [study engine](../apps/SelfHosted-AI_Tests/src/study.rs) |
| Step practice/corrections/mastery/journal | [learning workflows](../apps/SelfHosted-AI_Tests/src/learning.rs) |
| Live logs/cancellation | [runtime logs](../apps/SelfHosted-AI_Tests/src/runtime.rs) and study engine |
| Delete history | [history transactions](../apps/SelfHosted-AI_Tests/src/history.rs) |
| Public webpage import | [web source fetcher](../apps/SelfHosted-AI_Tests/src/web_source.rs) |
| Notes UI/Markdown | [notes frontend](../web/index.html) |
| Study inputs/artifacts/tutorial | [learning frontend](../apps/SelfHosted-AI_Tests/web/index.html) |

## Invariants to preserve

- One Node command manages one native server/listener. Notes and Learn are libraries, with in-memory integration.
- Human approval is the default before note publication. Explicit bulk Skip human intervention requests bypass questions/review without creating human correction lessons. There is no AI moderator or timeout approval.
- Old source snapshots, artifact revisions and attempts survive changes unless explicitly deleted.
- Notes retain ownership of approved Markdown and recorded vault paths; Learn retains ownership of study history.
- Never expose provider secrets through learning profile responses or runtime logs.
- Cancelled generation cannot publish a late result. Interrupted work is retryable after restart.
- Source sections are numbered outside code fences. Do not reveal answer keys or full notes in a restricted recall step.
- Markdown inputs retain their raw source. Preserve IME handling and Enter/Shift+Enter behavior.
- Keep a native element directly beneath MUI Snackbar transitions.
- User data belongs under ignored runtime directories, never committed fixtures. Test fixtures use temporary directories.

## Verification

Follow [development checks](DEVELOPMENT.md). The socket-free Rust integration tests cover nested routes, all-route authentication and in-memory notes configuration/import. UI harnesses cover behavioral contracts, not visual browser layout. Fake-model smoke tests require local listeners. Real providers and generated content require separate acceptance testing.
