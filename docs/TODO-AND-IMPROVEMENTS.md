# Todo & Suggested Improvements

Structured backlog for the ebook-converter project, with focus on the library adapter and related areas.

---

## Todo (actionable)

### Library adapter & config

- [ ] **HttpLibrary backend** – Implement `LibraryConnection` for the HTTP API described in `library-standard.md`: `GET /api/capabilities`, `GET /api/entries`, `GET /api/entries/{id}/file`, `PUT /api/entries`, `DELETE /api/entries/{id}`. Use `reqwest` (or existing HTTP client) with optional `Authorization` and API-key headers.
- [ ] **Extend LibraryConfig** – Add `url: Option<String>`, `path: Option<String>`, `auth: Option<LibraryAuth>` (e.g. `{ type = "bearer", token = "..." }` or `api_key = "..."`) so the CLI/config can choose HTTP vs directory backend and pass credentials.
- [ ] **Connection factory** – Add a function (e.g. in `library.rs` or `config.rs`) that builds a `Box<dyn LibraryConnection>` from `LibraryConfig`: if `url` is set → `HttpLibrary`, else if `path` is set → `DirLibrary` with optional `template`, else → `StubLibrary`.
- [ ] **CLI: pull from library** – Subcommand (e.g. `ebook-converter library pull [--id ID] [--output DIR]`) that uses the configured connection to list (or get by id), download file(s), and optionally convert and write to a path.
- [ ] **CLI: push to library** – Subcommand (e.g. `ebook-converter library push <file>`) that reads the file, optionally gets metadata, and calls `put()` on the configured library (using `output_dir` / template from config where relevant).
- [ ] **CLI: library list** – Subcommand to list entries from the configured library (with `--limit`, `--format`, `--query` mapping to `ListOptions`).

### DirLibrary

- [ ] **DirLibrary: recursive listing** – Option to scan subdirectories (e.g. `DirLibrary::new(root).recursive(true)`) and expose ids as relative paths so nested folders work as a single library.
- [ ] **DirLibrary: metadata on list** – When listing, optionally read metadata (e.g. EPUB title/author via existing readers) so `LibraryEntry.metadata` is populated instead of default; consider a `list_with_metadata: bool` or a separate `list_detailed` to avoid slow scans by default.
- [ ] **DirLibrary: path traversal safety** – Validate that `id` in `get`/`delete` does not escape `root` (e.g. `..` or absolute paths); reject with `LibraryError::Failed` or a dedicated variant.
- [ ] **DirLibrary: put overwrite** – Document or config option for overwrite vs unique naming (e.g. append number if file exists) to avoid accidental overwrites.

### Tests & docs

- [ ] **Unit tests for library** – Tests for `DirLibrary` (list/get/put/delete in a temp dir), `ListOptions` filtering, and `StubLibrary` behavior.
- [ ] **Integration test for HttpLibrary** – Once implemented, test against a mock server or recorded responses.
- [ ] **Doc examples** – Add `# Examples` in `library.rs` for `DirLibrary::new(...).with_put_template(...)` and a minimal `LibraryConnection` usage.

### Library server (separate app)

- [ ] **Library server spec** – Optional: more detailed spec (status codes, error JSON shape, pagination response format) in `library-standard.md` or a separate `library-api.md` for implementors.
- [ ] **Reference server** – Out of scope for this repo: a minimal server (e.g. Rust or other) that implements the standard for development and dogfooding.

---

## Suggested improvements

### Library & adapter

- **Capability discovery** – Have the CLI or GUI call `capabilities()` and grey out / hide unsupported actions (e.g. “Delete” when `delete: false`) instead of failing at call time.
- **Pagination** – Document or enforce that `ListResult.total` is set when the backend supports it so UIs can show “Page 1 of N” or “Load more”.
- **Search** – If `LibraryCapabilities.search` is true, consider a dedicated `search(&self, query: &str, opts: &ListOptions)` on the trait later; for now, `ListOptions.query` can remain the single mechanism.
- **Idempotent put** – Standard could allow “put with id” (e.g. `PUT /api/entries/{id}`) so the client can overwrite a known id; adapter trait could gain `put_with_id(&self, id: &str, ...)` as an optional extension.
- **Auth refresh** – If using OAuth or short-lived tokens, config or a callback for refreshing credentials before they expire (future improvement).

### DirLibrary

- **Symlinks** – Decide whether to follow symlinks in `list`/`get` or treat them as entries; currently behavior is filesystem-dependent.
- **File watcher** – Optional integration with `watch` module so that “library list” or a future UI can refresh when the directory changes (optional, may be overkill for CLI).
- **Large directories** – For very large dirs, consider streaming or iterator-based list to avoid loading all entries into memory (e.g. return a stream of `LibraryEntry` or chunked results).

### Config & CLI

- **Multiple libraries** – Support named libraries in config (e.g. `[libraries.home]`, `[libraries.cloud]`) and CLI flag `--library home` to choose which to use for pull/push.
- **Validation** – Validate `LibraryConfig` at load time (e.g. if both `url` and `path` set, prefer one or error) and on `config set`.
- **Library init** – Subcommand `ebook-converter library init --path ~/Books` that creates or updates config with `path` and template so users can get started quickly.

### General

- **Existing warnings** – Fix current compiler warnings: unused import in `readers/txt.rs`, unused variable in `encoding.rs`, unnecessary `mut` in `merge.rs` and `repair.rs`, dead code in `readers/epub.rs` (or allow/use the field).
- **Error context** – Add `.context()` or similar in library and convert paths so that chain of operations (e.g. “pull id X from library Y”) is visible in error messages.
- **Logging** – Add optional `log`/`tracing` for library operations (list count, get id, put result) to aid debugging without printing to stdout.

---

## Priority suggestion

1. **High**: HttpLibrary, config extension, connection factory, CLI pull/push/list – makes the adapter usable end-to-end.
2. **Medium**: DirLibrary path traversal safety, recursive option, metadata on list; unit tests for library.
3. **Lower**: Multiple libraries in config, server spec doc, logging, fix existing warnings.

---

*Last updated: 2025-02-22*
