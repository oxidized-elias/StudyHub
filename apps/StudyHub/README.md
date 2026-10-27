# StudyHub server and shell module

This module is the **single integrated executable** for the repository. It links Notes and Learn as Rust libraries, mounts their routers at `/notes` and `/learn`, and serves the unified Material UI shell. Notes-to-learning integration dispatches in memory; there are no separate backend processes or proxy targets.

From the repository, run `node server.mjs` and open `http://127.0.0.1:8082`. See the [main README](../../README.md), [architecture](../../docs/STUDY_SUITE.md), and [configuration](../../docs/CONFIGURATION.md). The old `--connect`, sibling installation and multi-server startup have been removed.

`src/main.rs` owns startup/authentication/routing; `web/index.html` owns navigation, notifications and trusted frame handoffs. Both feature frames remain mounted while switching sections. Every public route shares the same optional authentication boundary.
