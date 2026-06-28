# Changelog

This is the changelog for the `msqlx` hard fork. Upstream sqlx history is not
tracked here.

## Unreleased

### Redux — collapse to a Postgres-only client

Comprehensive hard fork of sqlx 0.9.0 down to a Postgres-only client suitable
for absorption into the `pg-client` crate. ~57% LOC reduction (82,103 → ~34,900).

**Subsystems removed.** mysql/sqlite/mariadb drivers and the `any` abstraction;
`sqlx-cli`; the migrate module (replaced by `mmigration`); sqlx-toml runtime
config; offline `.sqlx` query cache (planned replacement: `pg-ephemeral`
integrated into the macro); `advisory_lock`; the Drop-RAII transaction
abstraction (`Transaction` / `TransactionManager`); `PgListener` LISTEN/NOTIFY
wrapper; the connection pool (`Pool` / `PoolConnection` / `PoolOptions`); the
`testing` module and `#[sqlx::test(fixtures(...))]` advanced path; `PgCopyIn` /
COPY FROM-TO STDOUT (its Drop wrote a `CopyFail` wire message on incomplete
finish and panicked on serialize failure — Drop-as-rescue anti-pattern);
`.pgpass` file lookup; MD5 password authentication; all `PG*` environment
variable reading; filesystem-based Unix socket detection; `.env` file loading;
`QueryLogger` + `LogSettings` infrastructure (statement-time observability is
the caller's job via their own `tracing` spans); the `log` crate direct
dependency and the dual-emit (`log` + `tracing`) pattern — library code now
emits through `tracing` only.

**Abstractions collapsed.** The `Database` trait and its eleven associated
types concretized to their `Pg*` equivalents — the trait is now an empty
marker. `sqlx-core` merged into `sqlx-postgres` (transitional `sqlx_core`
module namespace retained). `sqlx-test` inlined into
`tests/postgres/common.rs`. The `postgres` cargo feature removed; Postgres is
mandatory. `PgConnection` is concrete throughout.

**Connection options.** `PgConnectOptions::new()` returns hardcoded defaults
(`localhost:5432` / `postgres` / `Prefer`). No implicit input from process
env, the filesystem, or `$HOME` — all configuration is explicit via the
builder or a parsed URL.

**Macros.** `DATABASE_URL` (or the configured override) must be set in the
process environment at macro-expansion time and at test runtime. Use direnv,
a shell alias, or any env-loading wrapper if convenient. No `.env` file
loading.

**Workspace.**
* Members: `msqlx` (facade), `sqlx-macros`, `sqlx-macros-core`,
  `sqlx-postgres`. Previously eleven members plus ten example crates.
* Runtime: `tokio` only.
* TLS: `rustls` + `ring` only.
* Optional type integrations retained: bigdecimal, bit-vec, chrono, ipnet,
  ipnetwork, mac_address, rust_decimal, time, uuid, bstr.

**Dropped dependencies (resolved out of `Cargo.lock`).** rsa
([RUSTSEC-2023-0071]), sha1, sqlx-cli's clap tree, serde+toml for sqlx.toml,
crossbeam-queue, event-listener, tempfile, criterion, futures-channel,
futures-io, cfg-if, indexmap, md-5, etcetera, tokio-stream, dotenvy, whoami,
log (direct), plus their transitive closures. ~40 packages pruned in total.

**Custom Drops.** Removed all of them: `PgCopyIn`, the pool's Drop fan-out,
`Transaction`, `QueryLogger`, and the dead-code `WriteAndFlush`. Zero custom
Drop impls remain.

[RUSTSEC-2023-0071]: https://rustsec.org/advisories/RUSTSEC-2023-0071.html
