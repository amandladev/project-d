# Finance Backend

A personal finance backend written in Rust, designed with an offline-first architecture. It can be embedded into mobile applications (iOS/Android) via FFI or exposed as an HTTP API.

## Features

- **Offline-first** — works entirely without network, syncs when available
- **Clean Architecture** — pure domain layer with no infrastructure dependencies
- **SQLite storage** — lightweight, local persistence with automatic migrations
- **Sync engine** — detects pending changes and resolves conflicts (Last Write Wins)
- **FFI ready** — C-compatible interface for Swift and React Native integration
- **Accounts, categories & transactions** — full financial tracking with computed balances

## Project Structure

| Crate | Purpose |
|-------|---------|
| `finance-core` | Domain entities, business rules, repository traits |
| `finance-storage` | SQLite implementation of repositories |
| `finance-sync` | Synchronization engine with conflict resolution |
| `finance-ffi` | C-compatible FFI layer for mobile integration |

## Getting Started

```bash
# Build
cargo build

# Run tests
cargo test
```

## License

MIT
