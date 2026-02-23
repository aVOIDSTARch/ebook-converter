# Ebook-Converter Project: Todo & Suggested Improvements

Project-wide backlog for the main ebook-converter (conversion pipeline, CLI, FFI, WASM, config, formats, and quality). For **library adapter**–specific items see [TODO-AND-IMPROVEMENTS.md](TODO-AND-IMPROVEMENTS.md).

---

## Todo (actionable)

### Conversion pipeline & formats

- [ ] **More readers** – Add readers for formats already detected: HTML, Markdown (and optionally PDF, FB2). Wire in `convert.rs` and `parse_format()` so convert CLI supports e.g. `--format md` / `--format html`. Readers mod already has commented placeholders for html, markdown, pdf.
- [ ] **More writers** – Add writers for HTML, Markdown, SSML (and optionally PDF). Wire in `convert.rs` and `parse_format()` so output formats match CLI help (epub, txt, html, md, pdf, ssml).
- [ ] **Align parse_format with Format** – Extend `parse_format()` to return all formats that have readers/writers (e.g. `html`, `md`, `ssml`, `pdf`) and document which combinations are supported (read × write matrix).
- [ ] **Convert: unsupported format message** – When detect returns a format with no reader (e.g. PDF), return a clear error suggesting “reading X is not yet supported” instead of a generic read error.
- [ ] **Config-driven convert** – Use `AppConfig` (e.g. `library.format`, `library.template`, `security.*`) in the convert path when CLI runs convert, so default output format and security limits come from config.

### CLI

- [ ] **Config set: encoding & watch** – Support `config set encoding.*` and `config set watch.*` in `set_config_key()` (e.g. `encoding.unicode_form`, `encoding.smart_quotes`, `watch.debounce_ms`). Today only library, lookup, security are handled.
- [ ] **Convert: use library template for rename** – When `convert --rename <template>` is used, apply the same template logic as rename subcommand (or delegate to `rename::format_title`) for output filename.
- [ ] **Progress reporting** – Pass an optional `ProgressHandler` (e.g. CLI progress bar or callback) into `read_document` / `write_document` for long-running convert/optimize so users see activity.
- [ ] **Batch convert output dir** – When multiple inputs and `--output <dir>` are given, ensure each output file is written under that directory with a unique name (already partially there; verify edge cases and document).
- [ ] **JSON output consistency** – Ensure all subcommands that support `--json` emit valid JSON only (no mix of log lines and JSON); consider a single helper for JSON vs human output.

### FFI

- [ ] **Stable error codes** – Document or define constants for return codes (e.g. `EBOOK_OK=0`, `EBOOK_ERR_NULL`, `EBOOK_ERR_INVALID_PATH`, `EBOOK_ERR_CONVERT`, etc.) and use them in `ebook_convert` and `ebook_validate` so C callers can branch on failure reason.
- [ ] **Additional FFI entry points** – Expose `ebook_repair`, `ebook_optimize`, and optionally `ebook_meta_get` / `ebook_meta_set` for embedding repair/optimize/metadata in other apps.
- [ ] **Error message buffer** – Optional `ebook_convert_ex(paths, ..., char* err_buf, size_t err_len)` to return a human-readable error string for logging/debugging.

### WASM

- [ ] **WASM: repair & validate options** – Expose `validate_ebook` options (strict, accessibility, wcag_level) and add `repair_ebook(data, format)` returning repaired bytes (or JSON report) for use in browser tools.
- [ ] **WASM: progress / cancel** – If needed for large files, support optional progress callback and abort signal so UI can show progress and cancel.
- [ ] **WASM: supported format list** – Export `supported_input_formats()` and `supported_output_formats()` (or a single list) so the host can show dropdowns without hardcoding.

### Config & security

- [ ] **Apply security config to reads** – When loading config, build `SecurityLimits` from `security.max_file_size_mb` and `security.max_compression_ratio` and pass into `ReadOptions` so CLI and FFI respect user limits.
- [ ] **Apply encoding config** – Build `EncodingOptions` from `encoding.*` and pass into `ReadOptions` (and any write path that uses encoding) so config controls normalization, smart quotes, etc.
- [ ] **Config validation** – Validate on load (e.g. numeric ranges, known keys for `config set`) and optionally on `config init` write a commented example.

### Lookup

- [ ] **Lookup: use config** – Use `LookupConfig` (default_provider, cache_dir, cache_ttl_hours) in the lookup CLI path instead of only `--provider openlibrary`.
- [ ] **Lookup: cache** – Implement optional file-based cache for provider responses using `cache_dir` and `cache_ttl_hours` so repeated lookups are fast.
- [ ] **Additional providers** – Add at least one more metadata provider (e.g. Google Books or a generic “by ISBN” interface) behind the same `MetadataProvider` trait.

### Watch

- [ ] **Implement or remove watch** – Either implement directory watch (e.g. using `notify` crate) that re-runs convert/repair on file changes with debounce from config, or remove the stub and config so the project does not imply support.

### Transforms

- [ ] **Built-in transforms** – Provide one or two built-in `Transform` implementations (e.g. “normalize encoding”, “strip metadata”) and wire them into write options or a CLI flag (e.g. `--transform encoding`).
- [ ] **Transform docs** – Document the `Transform` trait and how to plug custom transforms into `WriteOptions.transforms` for advanced users.

### Docs & polish

- [ ] **README** – Replace placeholder README with project name, short description, build/install, usage examples (convert, validate, repair, config), and links to docs (library standard, project todo, config keys).
- [ ] **Fix compiler warnings** – Resolve existing warnings: unused import in `readers/txt.rs`, unused variable in `encoding.rs`, unnecessary `mut` in `merge.rs` and `repair.rs`, dead code in `readers/epub.rs` (ManifestItem.properties).
- [ ] **Doc comments** – Add or expand module-level docs for `convert`, `detect`, `readers`, `writers`, and public functions used by CLI/FFI/WASM.

### Tests

- [ ] **Convert integration tests** – End-to-end tests: epub → txt, txt → epub, and optionally html/md once readers/writers exist; verify round-trip where meaningful.
- [ ] **CLI integration tests** – Run CLI with temp files for convert, validate, repair, config set/show and assert exit code and (where possible) output shape.
- [ ] **FFI tests** – Call `ebook_convert` and `ebook_validate` from C or a C test harness to ensure ABI and return codes are stable.
- [ ] **WASM tests** – Add `wasm-bindgen-test` (or Node) tests for `convert` and `validate_ebook` with small fixtures.

---

## Suggested improvements

### Conversion & formats

- **Read/write matrix doc** – Maintain a small table or doc (e.g. in README or `convert.rs`) listing supported input → output format pairs so users and contributors know what’s implemented.
- **Streaming for large files** – For very large ebooks, consider streaming read/write where the IR allows (e.g. chunked content) to cap memory use.
- **Format-specific options** – Allow per-format options in CLI (e.g. `--epub-version 3`, `--image-quality 80`) and pass through to `WriteOptions` / readers.

### CLI

- **Global config path** – Support `--config <path>` to load a specific config file for scripting or multiple profiles.
- **Quiet mode** – `-q` / `--quiet` to suppress “Converted: …” and other informational output while keeping errors.
- **Exit codes** – Document and stick to exit codes (0 = success, 1 = error, 2 = partial success if batch) for scripting.

### FFI / WASM

- **Single C header** – Ship a small `ebook_converter.h` with function declarations and error code defines for C/C++ consumers.
- **WASM size** – Track and optionally document WASM binary size; consider `opt-level="z"` and stripping unused format code if building a “minimal” WASM target.

### Config

- **Schema or example** – Provide `config.toml.example` or a schema so editors can validate and users can copy.
- **Environment overrides** – Optional env vars (e.g. `EBOOK_CONVERTER_CONFIG`) to override config path or key values for CI/containers.

### General

- **Error context** – Use `anyhow` or manual context in CLI/FFI so errors include operation context (e.g. “convert: reading input: …”).
- **Structured logging** – Use `tracing` spans for convert/read/write so debug logs can be filtered by operation; already using `tracing_subscriber` in CLI.
- **Changelog** – Keep a `CHANGELOG.md` for notable fixes and features for users and packagers.

---

## Priority suggestion

1. **High** – README and fix compiler warnings; config set for encoding/watch; apply security and encoding config to reads; align parse_format and document supported matrix.
2. **Medium** – More readers/writers (HTML, Markdown, SSML); FFI error codes and optional repair/optimize; WASM options and format list; lookup config and cache.
3. **Lower** – Watch implementation or removal; built-in transforms; streaming; C header; changelog; additional lookup provider.

---

*Last updated: 2025-02-22*
