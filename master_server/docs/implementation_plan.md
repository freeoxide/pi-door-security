# Master Server — Implementation Plan

This plan turns the MVP requirements in `refined_specs.md` into actionable engineering steps. Phases are ordered to maximize early feedback (API + DB foundations first) and to keep Dockerized delivery usable throughout.

## Phase 0 — Project Foundations
- Confirm repo layout (`master_server` crate, docs, docker assets).
- Set up Rust workspace and dependencies (`axum`, `sea-orm`, `argon2`, `jsonwebtoken` or session crate, `totp` helper, `serde`, `tokio`, etc.).
- Add linting/formatting (`cargo fmt`, `cargo clippy`) and CI placeholders.
- Prepare Dockerfile and docker-compose.yml skeleton referencing Postgres.
- Implement config loader (env vars with sensible defaults per refined specs).

## Phase 1 — Database Schema & Migrations
- Author SeaORM entities for: `users`, `clients`, `user_clients`, `sessions`, `events`, `commands`, `heartbeats`. Ensure `clients` captures `eth0_ip`, `wlan0_ip`, and shared `service_port`.
- Generate migrations covering tables, indexes, enums, constraints.
- Setup migration runner invoked at app startup.
- Provide seed script toggled by env (for dev/demo data).

## Phase 2 — Core Infrastructure
- Wire Axum app with shared state (`AppState`: DB pool, config, clock, etc.).
- Add middleware: request logging, JSON error mapping, auth extractor placeholder.
- Implement health endpoint (`GET /healthz`).
- Integrate structured logging (JSON) and optional Prometheus metrics stub.

## Phase 3 — Authentication & Sessions
- Implement password hashing/verification via Argon2id.
- Build login flow: `POST /auth/login` issuing opaque session tokens stored in DB.
- Create middleware/extractor for bearer token validation + role enforcement.
- Add logout endpoint (`POST /auth/logout`) to revoke tokens.
- Implement TOTP setup & verification endpoints; store OTP metadata.
- Extend migrations/entities if extra fields needed (e.g., token revocation timestamps).

## Phase 4 — User Management (Admin)
- CRUD endpoints for users (list, create, update role/password, delete).
- Authorization checks restricting operations to admin role.
- Unit/integration tests covering role enforcement and validation.

## Phase 5 — Client Lifecycle
- Endpoint to create clients with generated `provision_key` (admin only).
- List/get endpoints filtered by role (admins see all; users restricted to assignments).
- Assignment endpoints for linking/unlinking users to clients.
- Data validation for IP formats and shared service port (single port value applied to both interfaces).
- Implement `PATCH /clients/{id}/network` (admin/user scope) to update stored IPs/service port manually.

## Phase 6 — Client Registration & Tokens
- Implement `POST /clients/register` flow:
  - Validate `provision_key`, capture network data (two IPs + shared service port).
  - Issue client-specific API token; persist with scoping separate from user sessions.
  - Invalidate or rotate `provision_key` after use.
- Add client auth extractor for subsequent heartbeat/event requests.
- Allow clients to call `PATCH /clients/{id}/network` with their token to refresh network info when DHCP changes occur.

## Phase 7 — Telemetry (Heartbeats & Events)
- `POST /clients/{id}/heartbeat`: update `last_seen_at`, status, uptime.
- Background task to transition clients to `offline` if heartbeat stale.
- `POST /clients/{id}/events`: persist logs asynchronously (consider queue or direct insert with spawn-blocking).
- Expose status endpoints (`GET /clients/{id}/status`, events list with filters).

## Phase 8 — Command Dispatch
- Admin/user endpoint to issue commands (with validation on assigned clients).
- Client polling endpoint to fetch pending commands.
- Ack endpoint to update command status + error messages.
- Consider simple polling interval configuration and status metrics.

## Phase 9 — Admin Bootstrap CLI (`masterctl`)
- Implement CLI binary sharing SeaORM models/config.
- Provide `bootstrap-admin` command prompting for credentials and optional TOTP.
- Ensure CLI works inside container (`docker compose run ...`).

## Phase 10 — Docker & Ops Polish
- Finalize Dockerfile (multi-stage build) and docker-compose with Postgres volume.
- Ensure migrations run on container start (entrypoint script or app bootstrap).
- Add `.env.example` and documentation for runtime configuration.
- Document HTTPS expectations (reverse proxy setup guidance).

## Phase 11 — Testing & QA
- Unit tests: auth logic, validators, command transitions.
- Integration tests with in-memory or test Postgres container (e.g., `testcontainers`).
- End-to-end smoke workflow: user login, client creation, registration, heartbeat, command ack.
- Security reviews: rate limiting stubs, logging redaction, secret handling.

## Phase 12 — Documentation & Handoff
- Update `README.md` with setup, run, and API overview.
- Generate OpenAPI (optional but recommended) for REST endpoints.
- Sync docs (`refined_specs.md`, `implementation_plan.md`) with any scope changes.
- Prepare release checklist (build, test, docker compose up).

## Dependencies & Ordering Notes
- Phases 0–2 unblock all subsequent work; avoid progressing without DB + HTTP skeleton.
- Client registration (Phase 6) depends on client CRUD and assignments.
- Command dispatch relies on client auth + telemetry groundwork.
- CLI bootstrap requires migrations/entities finalized.
