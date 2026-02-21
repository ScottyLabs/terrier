# RFC 0006: Observability

- **Status:** Draft
- **Author(s):** @ap-1, @kritdass, @sreeram
- **Created:** 2026-02-21
- **Updated:** 2026-02-21

## Overview

This document describes the observability strategy for our application. It covers how telemetry data (traces, metrics, and logs) is collected, where it goes, and how each service in our stack is instrumented.

Our stack consists of:

- **Backend:** Axum (Rust)
- **Frontend:** Svelte
- **Database:** PostgreSQL (via SeaORM)
- **Cache / Session Store:** Valkey (Redis-compatible), via `fred` client
- **Object Storage:** MinIO
- **Auth:** `axum-oidc` with `tower-sessions` backed by Valkey

## Goals

- Capture errors and performance data in **Sentry** for engineering visibility
- Ship distributed traces to **Grafana Tempo** for deep infrastructure-level trace querying
- Expose metrics for eventual scraping by **Prometheus** (infrastructure is not yet set up, but instrumentation should be ready)
- Correlate logs with errors and traces inside Sentry
- Avoid vendor lock-in at the instrumentation layer — use OpenTelemetry as the spine

## Non-Goals

- Setting up Prometheus, Grafana, or Tempo infrastructure (out of scope for this RFC — addressed separately)
- Product analytics / user behavior tracking (PostHog — a separate decision not yet made)
- Defining alerting rules or SLOs

## Decisions

### 1. OpenTelemetry is the instrumentation spine

We use the OpenTelemetry SDK as the central instrumentation layer. This keeps our application code vendor-neutral. Sentry and Tempo are both destinations that receive data from the OTel pipeline, rather than being instrumented directly.

The data flow is:

```
tracing macros in application code
        ↓
tracing-opentelemetry (bridges tracing spans into OTel SDK)
        ↓
OTel SDK (span context, propagation)
        ↓  (fan out via processors)
        ├── SentrySpanProcessor  →  Sentry (errors + performance)
        └── OTLP Exporter  →  OTel Collector  →  Grafana Tempo
```

### 2. Sentry for errors, performance, and logs

Sentry is our primary engineering-facing observability tool. It captures:

- Errors and panics (with request context)
- Performance transactions (one per HTTP request)
- Structured logs correlated with errors and traces

Sentry receives trace data via the OTel integration (`SentrySpanProcessor`), not via its own `sentry-tracing` span management. This avoids conflicts between the two systems.

The `sentry-tracing` subscriber layer is still used, but **only for log forwarding** — its default `event_filter` (when the `logs` feature is enabled) correctly handles this without touching span lifecycle.

### 3. `tracing` is the instrumentation API

All application code uses the [`tracing`](https://docs.rs/tracing) crate for instrumentation: `#[instrument]` on functions, `info!`/`warn!`/`error!` macros for log events, and manual `span!` creation where needed. This is the single API used for both traces and logs throughout the codebase.

### 4. Metrics via `axum-prometheus`

HTTP-level metrics (request count, latency histograms, status code breakdown) are exposed via a `/metrics` endpoint using `axum-prometheus`. This is independent of the OTel pipeline and is ready for Prometheus to scrape when that infrastructure is stood up.

### 5. Valkey client: `fred`

`axum-oidc` uses `tower-sessions` for session storage, and `tower-sessions-redis-store` (which backs Valkey storage) uses `fred` as its Redis client. We use `fred` directly for any additional Valkey usage (e.g. pub/sub) to share the same client and connection pool rather than introducing a second Redis client crate.

## Per-Service Instrumentation

### Axum Backend

**Traces:** `tower-http`'s `TraceLayer` creates a span per HTTP request. `tracing-opentelemetry` bridges these into the OTel SDK. `traceparent` headers are propagated inbound (from Svelte frontend) and outbound (to any downstream calls).

**Errors:** Sentry Tower middleware (`NewSentryLayer` + `SentryHttpLayer`) binds a Sentry Hub per request and attaches HTTP metadata to transactions.

**Metrics:** `axum-prometheus` exposes a `/metrics` scrape endpoint.

**Logs:** `sentry-tracing` layer with `logs` feature forwards structured `tracing` events to Sentry as searchable logs.

**Cargo dependencies:**

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-opentelemetry = "0.28"
opentelemetry = "0.29"
opentelemetry_sdk = { version = "0.29", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.29", features = ["grpc-tonic"] }
tower-http = { version = "0.6", features = ["trace"] }
axum-prometheus = "0.7"
sentry = { version = "0.46", features = [
    "tower",
    "tower-axum-matched-path",
    "tracing",
    "logs",
    "opentelemetry",
] }
```

**Initialization order in `main` (important):**

1. Initialize Sentry (must happen before the async runtime starts)
1. Build the OTel `TracerProvider`, registering both `SentrySpanProcessor` and the OTLP exporter
1. Compose the `tracing-subscriber` registry: `fmt` layer (stdout) + `tracing-opentelemetry` layer + `sentry-tracing` layer
1. Build the Axum router, adding Tower middleware: `NewSentryLayer` → `SentryHttpLayer` → `TraceLayer` → `axum-prometheus`

### PostgreSQL

**Metrics:** The OTel Collector's `postgresql` receiver connects to Postgres and scrapes `pg_stat_*` views on a configurable interval. This requires a read-only monitoring user:

```sql
CREATE USER otel_monitor WITH PASSWORD '...';
GRANT pg_monitor TO otel_monitor;
```

OTel Collector config excerpt:

```yaml
receivers:
  postgresql:
    endpoint: localhost:5432
    username: otel_monitor
    password: ${env:PG_MONITOR_PASSWORD}
    databases: [your_db]
    collection_interval: 30s
    tls:
      insecure: false
```

Postgres does not natively emit OTLP. There is no agent to run alongside it for traces — trace context lives in the application layer.

**Traces:** SeaORM queries are instrumented at the application level. Query calls are wrapped in `tracing` spans so they appear as child spans nested under the HTTP request span in Tempo and Sentry. SeaORM sits on top of SQLx; there is no automatic instrumentation crate — this is done manually using `tracing::info_span!` or `#[instrument]` on repository/service functions.

### Valkey

**Metrics:** `redis_exporter` runs as a sidecar and exposes a Prometheus scrape endpoint. It connects to Valkey and reads `INFO` output.

**Traces:** `fred` (the client used by `tower-sessions-redis-store` and used directly for pub/sub) has a `tracing` feature flag that emits spans for client operations. Enable it in `Cargo.toml`:

```toml
fred = { version = "...", features = ["tracing"] }
```

This gives cache operation latency and hit/miss visibility as child spans in traces.

### MinIO

**Metrics:** MinIO exposes a native Prometheus endpoint at `/minio/metrics/v3`. No sidecar required. Configure an auth token and point Prometheus at it.

**Traces:** MinIO's OTLP trace support is limited to their enterprise AIStor product and is designed for support/diagnostics purposes rather than general observability. For open-source MinIO, S3 client calls are instrumented at the application layer — wrap `aws-sdk-s3` or `minio` SDK calls in `tracing` spans the same way as database queries.

### Svelte Frontend

**Errors + Performance:** Sentry JavaScript SDK. This captures frontend exceptions, Core Web Vitals, and performance transactions.

**Distributed tracing:** The Sentry JS SDK automatically attaches a `sentry-trace` header to `fetch`/`XHR` requests to the Axum backend. The Sentry Tower middleware on the backend reads this header and continues the trace, creating a fully connected trace from browser interaction through to database query.

Note: if at some point we want frontend traces flowing into Tempo as well, the OTel JS browser SDK can be added to emit `traceparent` headers instead, but this is not required now.

## What Is Not Yet Wired Up

These are instrumented (or ready to be) but require infrastructure to be set up before data flows anywhere:

| Signal | Source | Requires |
|---|---|---|
| Metrics (HTTP) | `axum-prometheus` `/metrics` endpoint | Prometheus scrape config |
| Metrics (Postgres) | OTel Collector `postgresql` receiver | OTel Collector deployment |
| Metrics (Valkey) | `redis_exporter` | Prometheus scrape config |
| Metrics (MinIO) | Native `/minio/metrics/v3` | Prometheus scrape config |
| Traces | OTLP exporter → Collector → Tempo | OTel Collector + Tempo deployment |

Sentry (errors, logs, and its own performance tracing) is the only destination that is fully operational without additional infrastructure.

## Open Questions

- **PostHog:** Not yet decided. Would be a separate frontend integration for product analytics. Does not overlap with this stack.
- **Log aggregation (Loki):** Logs currently go to Sentry only. If we want logs queryable independently (e.g. in Grafana via Loki), we would add a `tracing-subscriber` layer writing structured JSON to stdout and collect those with Promtail or Vector. This is additive and does not require changing application instrumentation.
- **OTel Collector deployment:** Not designed here. Needs its own config (receivers, processors, exporters for both Tempo and optionally Prometheus remote write).
