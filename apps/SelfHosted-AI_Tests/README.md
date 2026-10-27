# Learning module

Learn is a library inside StudyHub, with a Material UI frontend. It provides notebooks, source selection, guided chat/practice, quizzes, tests, flashcards, guides, generated activities, history, mastery, corrections and problem journals. It is no longer a separately launched server.

Run `node server.mjs` from the repository and open `http://127.0.0.1:8082`. See the [user guide](../../docs/USER_GUIDE.md), [learning workflows](../../docs/LEARNING_WORKFLOWS.md), [architecture](../../docs/STUDY_SUITE.md), [API](../../docs/API.md), and [feature coverage](FEATURES.md).

`src/lib.rs` initializes SQLite and exposes the router. It receives the Notes router for in-memory approved-source imports and settings. `providers.rs` adapts model protocols; `study.rs` owns sessions/tools/jobs/attempts; `learning.rs` owns tutoring/correction/mastery workflows; `history.rs` owns deletion; `runtime.rs` owns live logs; `web_source.rs` imports public pages. The frontend remains in `web/index.html`.

The historical directory/package name is retained for source navigation. Storage defaults to the main application's `data/learning/`, not a sibling project directory. Model hosts may be remote; the application server is unified.
