# Ebook Library Server

HTTP server that implements the [ebook library standard](../../docs/library-standard.md). Use it to expose a directory of ebooks over the web so that:

- **Websites** can list, download, and upload ebooks (CORS is enabled by default).
- **ebook-converter** CLI (with the HttpLibrary adapter) can pull and push books.

## Build and run

```bash
cargo build -p ebook-converter-library-server --release
./target/release/library-server
```

Or run with env overrides:

```bash
# Store ebooks in a specific directory (default: platform data dir or ./library)
export EBOOK_LIBRARY_PATH=/path/to/ebooks

# Bind address (default: 127.0.0.1:3030)
export EBOOK_LIBRARY_BIND=0.0.0.0:3030

./target/release/library-server
```

## API (library standard)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/capabilities` | Supported features (list, get, put, delete, search). |
| GET | `/api/entries` | List entries. Query: `?page=1&limit=50&q=...&format=epub`. |
| GET | `/api/entries/{id}` | Get entry metadata only. |
| GET | `/api/entries/{id}/file` | Download file bytes (Content-Type set by format). |
| PUT | `/api/entries` | Upload file (raw body → new id assigned). |
| DELETE | `/api/entries/{id}` | Remove entry. |

Storage is directory-backed: list scans the library directory; get/put/delete read/write files. Metadata for EPUB and TXT is read via **ebook-converter-core** when listing.

## Website integration

- Enable CORS (default: permissive) so browser apps on another origin can call the API.
- Use `GET /api/entries` for catalog listing and `GET /api/entries/{id}/file` for download links.
- Use `PUT /api/entries` with the ebook file as raw body to upload (e.g. from a form or drag-and-drop).

## Relation to ebook-converter

- The **library-server** is a standalone binary; it does not live inside the ebook-converter CLI.
- The **HttpLibrary** adapter (in ebook-converter-core, when implemented) will call this API so that `ebook-converter library pull` and `library push` work against a running server.
