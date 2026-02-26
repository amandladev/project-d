# Architecture

## Overview

This is an **offline-first personal finance backend** written in Rust, designed to be embedded into mobile applications (iOS via Swift, or React Native) through FFI. It follows **Clean Architecture** principles to keep the domain pure and technology-agnostic.

## Workspace Structure

```
project-d/
├── Cargo.toml                    # Workspace root
├── ARCHITECTURE.md               # This file
│
├── finance-core/                 # Pure domain layer
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── errors.rs             # Domain error types
│       ├── entities/
│       │   ├── mod.rs
│       │   ├── common.rs         # BaseEntity, SyncStatus, TransactionType
│       │   ├── user.rs
│       │   ├── account.rs
│       │   ├── category.rs
│       │   └── transaction.rs
│       ├── repositories/         # Trait definitions (ports)
│       │   ├── mod.rs
│       │   ├── user_repository.rs
│       │   ├── account_repository.rs
│       │   ├── category_repository.rs
│       │   └── transaction_repository.rs
│       └── use_cases/
│           ├── mod.rs
│           ├── account_use_cases.rs
│           ├── category_use_cases.rs
│           └── transaction_use_cases.rs
│
├── finance-storage/              # SQLite persistence (adapters)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── database.rs           # Connection + WAL + migrations
│       ├── error.rs
│       ├── migrations.rs         # Schema creation & indexes
│       └── repositories/
│           ├── mod.rs
│           ├── sqlite_user_repository.rs
│           ├── sqlite_account_repository.rs
│           ├── sqlite_category_repository.rs
│           └── sqlite_transaction_repository.rs
│
├── finance-sync/                 # Synchronization engine
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # SyncEngine, SyncTransport trait, LWW
│       └── tests.rs              # Sync tests with mock transport
│
└── finance-ffi/                  # FFI layer for mobile
    ├── Cargo.toml
    └── src/
        └── lib.rs                # extern "C" functions
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Mobile App (Swift / RN)                   │
└──────────────────────────┬──────────────────────────────────┘
                           │ FFI (C-compatible)
┌──────────────────────────▼──────────────────────────────────┐
│                      finance-ffi                            │
│  init_database · create_account · create_transaction        │
│  get_balance · get_pending_sync · free_string               │
└──────┬───────────────────┬──────────────────┬───────────────┘
       │                   │                  │
       ▼                   ▼                  ▼
┌──────────────┐  ┌────────────────┐  ┌───────────────┐
│ finance-core │  │finance-storage │  │ finance-sync  │
│              │  │                │  │               │
│  Entities    │  │  SQLite repos  │  │ SyncEngine    │
│  Repos(trait)│◄─│  Database      │  │ SyncTransport │
│  Use Cases   │  │  Migrations    │  │ LWW resolver  │
│  Errors      │  │                │  │               │
└──────────────┘  └────────────────┘  └───────────────┘
       ▲                                      │
       │           Domain depends on          │
       │              NOTHING external        │
       └──────────────────────────────────────┘
```

## Key Design Decisions

### 1. Amounts in Cents (i64)
All monetary amounts are stored as integers in cents (e.g., `$10.50` = `1050`). This avoids floating point precision issues that are critical in financial applications.

### 2. Balance Computed from Transactions
Account balance is **never stored directly** — it is always calculated from the sum of active (non-deleted) transactions. This ensures data integrity and prevents desync.

### 3. Soft Delete
Entities are never physically removed. A `deleted_at` timestamp marks them as deleted, preserving history and enabling sync of deletions to the server.

### 4. Offline-First with Sync Status
Every syncable entity carries:
- `sync_status`: `pending` | `synced` | `conflicted`
- `version`: integer for conflict detection

### 5. Last Write Wins (LWW)
Conflict resolution uses a simple LWW strategy based on version numbers. The entity with the higher version wins.

### 6. Repository Pattern
Domain repositories are defined as **traits** in `finance-core`. Concrete implementations live in `finance-storage`, keeping the domain completely free of infrastructure concerns.

## Complete Flow Example

```
1. User creates a transaction (offline)
   → Transaction created with sync_status = "pending", version = 1
   → Stored in local SQLite

2. App triggers sync
   → SyncEngine.collect_pending_changes() gathers all pending entities
   → SyncEngine.serialize_payload() converts to JSON
   → SyncTransport.push() sends to remote server

3. Server responds
   → Accepted IDs → mark as sync_status = "synced"
   → Conflicts → resolve using LWW (higher version wins)

4. Pull server changes
   → SyncTransport.pull() fetches new server data
   → Apply changes locally using LWW

5. Sync complete
   → All entities now in sync
   → Ready for next offline cycle
```

## FFI Error Handling

All FFI functions return a JSON string with this structure:

```json
{
  "success": true,
  "code": 0,
  "message": "OK",
  "data": { ... }
}
```

Error codes:
| Code | Meaning |
|------|---------|
| 0    | Success |
| 1    | Database not initialized |
| 2    | Invalid input (null pointer, bad UTF-8) |
| 3    | Invalid parameter (bad UUID, bad date) |
| 4    | Serialization error |
| 10   | Database open failed |
| 11   | Database already initialized |
| 20   | Account operation failed |
| 21   | Category operation failed |
| 22   | Transaction operation failed |
| 23   | Balance query failed |
| 30   | Sync query failed |

## Future Extensibility

- **finance-api**: Add an HTTP API using Axum that shares the same domain and storage layers.
- **Multi-device sync**: Replace the mock `SyncTransport` with a real HTTP client.
- **Recurring transactions**: Add a recurring transactions scheduler in the domain.
- **Reports/Analytics**: Add use cases for spending summaries, category breakdowns, trends.
- **Multi-currency**: Extend with exchange rate support.
