# Ebook Converter

CLI and library for converting, validating, repairing, and managing ebooks. Supports EPUB and plain text (more formats planned). Configuration is read from `~/.config/ebook-converter/config.toml`.

## Build and install

```bash
cargo build --release
# Binary: target/release/ebook-converter
```

Install to a prefix (optional):

```bash
cargo install --path crates/cli
```

## Usage

**Convert** (e.g. to EPUB or TXT):

```bash
ebook-converter convert input.txt -o output.epub -f epub
ebook-converter convert book.epub -o out/ -f txt
```

**Validate** structure and optional accessibility (WCAG):

```bash
ebook-converter validate book.epub
ebook-converter validate book.epub --accessibility --wcag-level AA
```

**Repair** common issues and write a fixed file:

```bash
ebook-converter repair book.epub -o book_fixed.epub
```

**Info** (metadata and stats):

```bash
ebook-converter info book.epub
```

**Rename** files with a template:

```bash
ebook-converter rename *.epub --template "{author} - {title}.{ext}" --dry-run
```

**Metadata** get/set/strip:

```bash
ebook-converter meta book.epub --get title
ebook-converter meta book.epub --set "title=My Book" "author=Author"
ebook-converter meta book.epub --strip
```

**Lookup** metadata from Open Library and apply:

```bash
ebook-converter lookup book.epub --provider openlibrary --apply
```

**Merge** / **Split** / **Dedup**:

```bash
ebook-converter merge a.epub b.epub -o combined.epub
ebook-converter split book.epub --by chapter --outdir chapters/
ebook-converter dedup ./library/ --strategy fuzzy
```

**Config** (init, show, set):

```bash
ebook-converter config init
ebook-converter config show
ebook-converter config set library.template "{author} - {title}.{ext}"
ebook-converter config set security.max_file_size_mb 500
ebook-converter config set encoding.unicode_form NFC
```

Use `--json` for machine-readable output where supported.

## Supported formats (current)

| Input  | Output | Notes |
|--------|--------|--------|
| EPUB   | EPUB, TXT | Full support |
| TXT    | EPUB, TXT | UTF-8, optional BOM |

Additional formats (HTML, Markdown, PDF, etc.) are detected but readers/writers are not yet implemented; see [docs/PROJECT-TODO-AND-IMPROVEMENTS.md](docs/PROJECT-TODO-AND-IMPROVEMENTS.md).

## Configuration

Config path: `~/.config/ebook-converter/config.toml`.

- **library**: `format`, `output_dir`, `template` (for default output format and naming).
- **security**: `max_file_size_mb`, `max_compression_ratio` (ZIP/archive limits).
- **encoding**: `unicode_form` (NFC, NFD, NFKC, NFKD), `smart_quotes`, `normalize_ligatures`, `fix_macos_nfd`.
- **lookup**: `default_provider`, `cache_dir`, `cache_ttl_hours`.
- **watch**: `debounce_ms`, `ignored_patterns`.

## Docs and roadmap

- [Library standard and adapter](docs/library-standard.md) – design for connecting to ebook libraries (web or local).
- [Project todo and improvements](docs/PROJECT-TODO-AND-IMPROVEMENTS.md) – conversion pipeline, CLI, FFI, WASM, config.
- [Library adapter todo](docs/TODO-AND-IMPROVEMENTS.md) – HttpLibrary, CLI pull/push, DirLibrary.

## Crates

- **ebook-converter** (CLI) – binary and subcommands.
- **ebook-converter-core** – conversion, validate, repair, meta, lookup, library adapter, security.
- **ebook-converter-ffi** – C-ABI for embedding (e.g. `ebook_convert`, `ebook_validate`).
- **ebook-converter-wasm** – WASM bindings for in-browser convert/validate.
- **ebook-converter-library-server** – HTTP server implementing the [library standard](docs/library-standard.md) for websites and the converter (list, get, put, delete ebooks). See [crates/library-server/README.md](crates/library-server/README.md).

## License

[Set as appropriate for your project.]
