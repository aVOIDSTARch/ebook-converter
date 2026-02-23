# Ebook Library Standard & Adapter Design

This document describes a **library standard** that an ebook library application (server) can implement, and the **adapter** in ebook-converter that talks to any compliant library over the web or locally.

## Goals

- **Library app**: A separate application/server that owns a collection of ebooks. It can run locally (e.g. desktop app with HTTP API) or be a web service.
- **Standard**: A minimal contract (HTTP API + semantics) so that any server implementing it can be used by ebook-converter.
- **Adapter**: Code in ebook-converter that discovers libraries, connects, and performs **pull** (fetch books into converter) and **push** (send converted/edited books into the library) using a single trait so backends (HTTP, local dir, future protocols) are pluggable.

## What a library server could do

| Capability | Description | Required? |
|------------|-------------|-----------|
| **List** | List entries (id, metadata, format, size). Support pagination and optional search/filter. | Yes |
| **Get** | Download a single ebook by id (returns bytes + format/content-type). | Yes |
| **Put** | Upload an ebook (bytes + suggested id or filename). Server may assign id. | Yes (for push) |
| **Delete** | Remove an ebook by id. | Optional |
| **Search** | Query by title, author, ISBN, etc. (may be same as list with query params). | Optional |
| **Metadata** | Get or update metadata only (without re-uploading file). | Optional |
| **Cover** | Get cover image by book id. | Optional |
| **Sync** | Report last-modified or checksums for sync/change detection. | Optional |
| **Auth** | API key, OAuth, or basic auth for remote libraries. | Optional (required for remote) |

For a minimal “v0” standard, **List**, **Get**, and **Put** are enough for pull and push.

## Library entry (summary)

Each item in the library is a **LibraryEntry**:

- **id**: Opaque string (server-defined).
- **metadata**: Title, authors, language, ISBN, etc. (align with our `Metadata` or a subset).
- **format**: epub, txt, etc. (our `Format` or MIME).
- **size_bytes**: File size if known.
- **updated_at**: Optional timestamp or version for sync.

The server can return a list of these (with optional pagination) and allow fetching the file by `id`.

## Transport: HTTP API (recommended for “web or local”)

A library server can expose a REST-like API that the adapter calls. Example shape:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/capabilities` | Returns supported features (list, get, put, delete, search, etc.). |
| GET | `/api/entries` | List entries. Query: `?page=1&limit=50&q=...&format=epub`. |
| GET | `/api/entries/{id}` | Get metadata only (optional). |
| GET | `/api/entries/{id}/file` | Get file bytes (Content-Type and Content-Disposition optional). |
| PUT | `/api/entries` or `/api/entries/{id}` | Upload file (multipart or raw body). |
| DELETE | `/api/entries/{id}` | Remove entry (optional). |

Authentication: `Authorization: Bearer <token>` or API key header. Local server may allow no auth.

## Local library

A “local” library can be:

- The same HTTP API bound to `localhost` (or a Unix socket), or
- A **directory-based** backend: a folder of ebooks (e.g. `~/Books`). The adapter treats it as read-only or read-write: list = scan dir; get = read file; put = write file with naming from config (e.g. `LibraryConfig.template`).

So the adapter can have two (or more) backends:

1. **HttpLibrary** – talks to a remote or local HTTP server that implements the standard.
2. **DirLibrary** – talks to a local directory (no server; adapter does filesystem ops).

## Adapter in ebook-converter

- **Trait**: `LibraryConnection` (or `EbookLibrary`) with methods: `capabilities()`, `list()`, `get()`, `put()`, and optionally `delete()`, `search()`.
- **Types**: `LibraryEntry`, `LibraryCapabilities`, `ListOptions`, and a `LibraryError` for failures.
- **Config**: Existing `LibraryConfig` can gain `url` (for HTTP) and/or `path` (for dir), plus `auth` (token, API key) when needed.

The CLI (or a future GUI) can then:

- **Pull**: “Import from library” → list entries, user picks one (or many) → get file(s) → optionally convert and save to a path.
- **Push**: After convert/repair, “Send to library” → put file (and maybe metadata) to the configured library.

No server implementation lives in ebook-converter; only the adapter and the contract (this doc + the trait and types) so that a separate library app can be built to the same standard later.
