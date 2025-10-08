# Master Server — Refined Specs (MVP)

Goal: deliver a minimal‑config, production‑leaning MVP that “just works” in Docker while exposing a clean REST API for admin and client operations. Keep defaults sensible; prefer convention over configuration.

## Scope

- Master server for managing and monitoring client servers (e.g., Pi door devices).
- Store users, clients, assignments, events/logs, commands, and uptime in Postgres via SeaORM.
- Provide REST API for admin operations, logs, configs, and command dispatch to clients.
- Support client registration with a pre‑provisioned key (UUIDv7) and submitting local IP:port for `eth0` and `wlan0`.
- Basic roles: `admin`, `user`.
- Auth: username + password plus optional TOTP (authenticator app). Long‑expiry session tokens.

Non‑goals (MVP):
- No multi‑tenant orgs/groups beyond user→client assignments.
- No complex RBAC; only `admin` vs `user`.
- No high‑availability orchestration; single DB and single master node (container) is fine.

## Tech Stack

- Language: Rust
- ORM: SeaORM (with migrations)
- DB: Postgres
- Web/API: Axum
- Auth: password hashing (Argon2), TOTP (RFC 6238)
- Tokens: signed JWT or opaque session tokens stored in DB (MVP: opaque tokens)
- Containerization: Docker + docker‑compose

## Deployment

- Services:
  - `master_server` (Rust binary) exposing HTTP API on `:8080`.
  - `postgres` with a mounted volume.
- Minimal config via environment variables:
  - `DATABASE_URL` (e.g., `postgres://postgres:postgres@postgres:5432/master`)
  - `SERVER_BIND` (default `0.0.0.0:8080`)
  - `TOKEN_TTL_HOURS` (default `720` i.e., 30 days)
  - `OTP_REQUIRED` (default `false`)
- One‑shot admin bootstrap via an interactive CLI (binary inside the image) to create the first `admin` user.

## Data Model (SeaORM Entities)

Use singular entity names; indices as noted.

- `users`
  - `id` (uuid, pk)
  - `username` (text, unique, index)
  - `password_hash` (text)
  - `role` (enum: `admin` | `user`, index)
  - `otp_secret` (text, nullable)
  - `otp_enabled` (bool, default false)
  - `created_at` (timestamptz, default now)

- `clients`
  - `id` (uuid, pk)
  - `label` (text)
  - `provision_key` (uuidv7, unique) — used once during initial registration
  - `eth0_ip` (inet, nullable)
  - `eth0_port` (int, nullable)
  - `wlan0_ip` (inet, nullable)
  - `wlan0_port` (int, nullable)
  - `status` (enum: `unknown` | `online` | `offline`, index)
  - `last_seen_at` (timestamptz, nullable)
  - `created_at` (timestamptz, default now)

- `user_clients` (assignment)
  - `user_id` (uuid, fk→users)
  - `client_id` (uuid, fk→clients)
  - pk: `(user_id, client_id)`
  - index on `client_id`

- `sessions`
  - `id` (uuid, pk)
  - `user_id` (uuid, fk→users, index)
  - `token` (text, unique) — opaque bearer token
  - `expires_at` (timestamptz, index)
  - `created_at` (timestamptz)
  - `revoked_at` (timestamptz, nullable)

- `events`
  - `id` (bigserial, pk)
  - `client_id` (uuid, fk→clients, index)
  - `ts` (timestamptz, index)
  - `level` (enum: `info` | `warn` | `error`)
  - `kind` (text)
  - `message` (text)
  - `meta` (jsonb, nullable)

- `commands`
  - `id` (uuid, pk)
  - `client_id` (uuid, fk→clients, index)
  - `issued_by` (uuid, fk→users, index)
  - `ts_issued` (timestamptz)
  - `command` (text)
  - `params` (jsonb, nullable)
  - `status` (enum: `pending` | `sent` | `acked` | `failed`, index)
  - `ts_updated` (timestamptz)
  - `error` (text, nullable)

- `heartbeats`
  - `id` (bigserial, pk)
  - `client_id` (uuid, fk→clients, index)
  - `ts` (timestamptz, index)
  - `uptime_ms` (bigint, nullable)

Notes:
- Compute uptime as cumulative difference between heartbeats while `online`; derive rollups as needed.
- Use database enums where appropriate or text + check constraints for simpler migrations.

## Authentication & Authorization

- Passwords hashed with Argon2id.
- Optional TOTP:
  - `POST /auth/otp/setup` returns URI/secret for authenticator apps; requires password re‑auth.
  - `POST /auth/otp/verify` enables OTP for the user after a valid code.
- Long‑expiry opaque bearer tokens:
  - `POST /auth/login` returns `{ token, expires_at }` after password (+OTP if enabled) verification.
  - Tokens stored in `sessions`; check on every request via `Authorization: Bearer <token>`.
- Roles:
  - `admin`: manage users, clients, assignments, configurations.
  - `user`: read/access assigned clients; can send allowed commands to those clients.

## REST API (MVP)

Auth
- `POST /auth/login` { username, password, otp_code? } → { token, expires_at }
- `POST /auth/logout` (auth) → 204
- `POST /auth/otp/setup` (auth) → { otpauth_uri, secret }
- `POST /auth/otp/verify` (auth) { code } → { otp_enabled: true }

Users (admin)
- `POST /users` { username, password, role } → user
- `GET /users` → [user]
- `PATCH /users/{id}` → user
- `DELETE /users/{id}` → 204

Clients
- `POST /clients` (admin) { label } → { id, provision_key }
- `GET /clients` (auth) → [client] (admins see all; users see assigned)
- `GET /clients/{id}` (auth) → client (must be assigned or admin)
- `DELETE /clients/{id}` (admin) → 204
- `POST /clients/{id}/assign` (admin) { user_id } → 204
- `DELETE /clients/{id}/assign/{user_id}` (admin) → 204

Client Registration & Telemetry (client → master)
- `POST /clients/register` { provision_key, eth0_ip?, eth0_port?, wlan0_ip?, wlan0_port? }
  → { client_id, api_token } (one‑time; invalidates `provision_key` and issues a client API token)
- `POST /clients/{id}/heartbeat` (client auth) { uptime_ms? } → 204
- `POST /clients/{id}/events` (client auth) { level, kind, message, meta? } → 202

Commands
- `POST /clients/{id}/commands` (auth) { command, params? } → command
- `GET /clients/{id}/commands?status=pending` (client auth) → [command]
- `POST /clients/{id}/commands/{cmd_id}/ack` (client auth) { success, error? } → 204

Logs & Status
- `GET /clients/{id}/events?since=...&level=...` (auth) → [event]
- `GET /clients/{id}/status` (auth) → { status, last_seen_at, eth0, wlan0 }

Configs (MVP minimal)
- `GET /clients/{id}/config` (auth) → { ... }
- `PATCH /clients/{id}/config` (admin) { ... } → { ... }

Notes:
- JSON everywhere; timestamps are ISO 8601 (UTC).
- For API pagination, use simple `limit` + `cursor` query parameters for lists (MVP optional).

## Client ↔ Master Protocol (MVP)

Registration
- Admin creates client: server returns `provision_key` (uuidv7).
- Client starts with `provision_key` and POSTs network info. Master returns `client_id` and a `client_api_token` (opaque) used for subsequent client calls.

Heartbeat & Status
- Client sends `POST /clients/{id}/heartbeat` every 30 seconds with optional `uptime_ms`.
- Master updates `last_seen_at` and flips `status` to `online` on receipt. A background task marks clients `offline` if no heartbeat for N minutes (e.g., 2× interval).

Command Delivery (simple & robust)
- Server stores commands in `commands` with status `pending`.
- Client polls `GET /clients/{id}/commands?status=pending` on a short interval (MVP). Optionally upgrade to WebSocket later.
- Client executes and ACKs with success/error. Server updates status.

## Admin Bootstrap CLI

- Binary: `masterctl`
- Run inside container interactively: `docker compose run --rm master_server masterctl bootstrap-admin`
- Flow prompts for `username`, `password`, optional `otp` setup, and creates first `admin`.

## Minimal Configuration Philosophy

- Sensible defaults for ports, token TTLs, and Docker networking.
- Single `.env` (optional). If missing, use defaults and start.
- No required manual DB prep; run migrations automatically on startup.

## Security Considerations

- Use HTTPS/TLS in production (terminated by reverse proxy like Caddy/Traefik).
- Rate‑limit auth endpoints; lockout or backoff after repeated failures.
- Store only hashed passwords; never log secrets or tokens.
- Scope tokens: user tokens vs client tokens stored separately.
- Validate IP/port inputs; store as typed fields (inet/int) to prevent injection.

## Observability

- Structured logs (JSON) for server.
- Basic metrics endpoint (e.g., `/metrics`) for Prometheus (optional for MVP).
- Event types standardized: `door.open`, `door.close`, `sensor.alert`, etc.

## Migrations & Seeding

- Use SeaORM migrations for schemas and enums.
- Optional seed command to create demo users/clients in non‑prod.

