#!/usr/bin/env python3
"""Generate a route/handler inventory from the actual Rust router declarations."""
import argparse
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
MODULES = [('Notes', '/notes', 'src/lib.rs'), ('Learning', '/learn', 'apps/SelfHosted-AI_Tests/src/lib.rs')]

def routes(source):
    for match in re.finditer(r'\.route\(\s*"([^"]+)"\s*,', source):
        start = match.end()
        # Route expressions contain no raw strings; count balanced delimiters to
        # avoid truncating chained get(...).put(...).delete(...) declarations.
        depth, end, quoted, escaped = 1, start, False, False
        while depth and end < len(source):
            char = source[end]
            if quoted:
                if escaped: escaped = False
                elif char == '\\': escaped = True
                elif char == '"': quoted = False
            elif char == '"': quoted = True
            elif char == '(': depth += 1
            elif char == ')': depth -= 1
            end += 1
        expression = ' '.join(source[start:end-1].strip().rstrip(',').split())
        methods = list(dict.fromkeys(m.upper() for m in re.findall(r'\b(get|post|put|delete|patch|head|options)\s*\(', expression)))
        yield match[1], ', '.join(methods), expression.replace('|', '&#124;')

def render():
    text = '''# API reference

Generated route/handler inventory. Regenerate with `python3 scripts/document-api.py`; CI checks it with `--check`. Paths below include the unified public prefix. GET routes also support HEAD through Axum. Authentication, when enabled, applies to every route, including shared materials. This is a trusted single-user API, with no per-user authorization.

## Conventions

JSON requests use `Content-Type: application/json`. Successful handlers return JSON unless they explicitly return HTML, source bytes or exports. Module errors normally return HTTP 400 with `{"error":"message"}`; authentication returns 401. Missing routes use 404. IDs are server-generated integers. Multipart uploads have a 64 MiB request limit. The handler and its request type are the authoritative contract for fields and validation.

## Shell

| Method | Path | Result |
| --- | --- | --- |
| GET | `/` | Unified HTML workspace. |
| GET | `/api/health` | `service`, `ok`, `unified`, `notes`, `learning`. A responding unified server has both initialized modules. |
| GET | `/notes/`, `/learn/` | Redirect to each module root. |

'''
    for title, prefix, filename in MODULES:
        source = (ROOT / filename).read_text()
        text += f'## {title}\n\nSource: [{filename}](../{filename}). Handlers in child modules are referenced by their module name.\n\n| Methods | Public route | Registration / handler |\n| --- | --- | --- |\n'
        for path, methods, handler in routes(source):
            text += f'| {methods} | `{prefix + path if path != "/" else prefix}` | `{handler}` |\n'
        text += '\n'
    text += '''## Common request examples

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
'''
    return text

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    options = parser.parse_args()
    destination = ROOT / 'docs/API.md'
    content = render()
    if options.check:
        if not destination.exists() or destination.read_text() != content:
            raise SystemExit('docs/API.md is stale; run python3 scripts/document-api.py')
        print('API documentation matches registered routes.')
    else:
        destination.write_text(content)
