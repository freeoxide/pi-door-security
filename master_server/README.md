# Master Server

Production-ready master server for managing and monitoring Pi door security client agents.

## Implementation Status

### ✅ Phase 0-2 Completed (Foundations & Core Infrastructure)

- [x] Rust workspace with all dependencies configured
- [x] Configuration system with sensible defaults
- [x] SeaORM database entities for all tables
- [x] Database migrations for complete schema
- [x] Axum web server with health endpoint
- [x] Structured JSON logging with tracing
- [x] Docker multi-stage build
- [x] Docker Compose setup with PostgreSQL
- [x] CLI binary skeleton (`masterctl`)

### 🚧 Next Phases (3-10)

**Phase 3:** Authentication & Sessions (password hashing, login, TOTP)
**Phase 4:** User Management (CRUD endpoints for admin)
**Phase 5:** Client Lifecycle (create, list, assign)
**Phase 6:** Client Registration & Tokens
**Phase 7:** Telemetry (heartbeats & events)
**Phase 8:** Command Dispatch
**Phase 9:** Admin Bootstrap CLI
**Phase 10:** Final Docker & Ops Polish

## Quick Start

### Development

```bash
# Copy environment template
cp .env.example .env

# Start with Docker Compose
docker compose up --build

# Health check
curl http://localhost:8080/healthz
```

### Running Locally

```bash
# Set up PostgreSQL (via Docker or local install)
docker run -d \
  --name postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=master \
  -p 5432:5432 \
  postgres:16-alpine

# Run the server
cargo run --bin master_server

# Run the CLI
cargo run --bin masterctl bootstrap-admin
```

## Database Schema

The following tables are created automatically via migrations:

- **users**: Admin and user accounts with role-based access
- **clients**: Pi door devices with network info and status
- **user_clients**: Assignments between users and clients
- **sessions**: Opaque bearer tokens for authentication
- **events**: Client event logs (structured logging)
- **commands**: Command queue for client dispatch
- **heartbeats**: Client uptime and health tracking

All migrations run automatically on server startup.

## Configuration

Environment variables (see [.env.example](.env.example)):

| Variable          | Default                                        | Description                  |
|-------------------|------------------------------------------------|------------------------------|
| `DATABASE_URL`    | `postgres://postgres:postgres@localhost:5432/master` | PostgreSQL connection string |
| `SERVER_BIND`     | `0.0.0.0:8080`                                 | Server bind address          |
| `TOKEN_TTL_HOURS` | `720` (30 days)                                | Session token TTL            |
| `OTP_REQUIRED`    | `false`                                        | Require TOTP for all users   |
| `RUST_LOG`        | `master_server=debug,tower_http=debug`         | Logging level                |

## Project Structure

```
master_server/
├── src/
│   ├── app.rs           # Axum router and state
│   ├── config.rs        # Configuration loader
│   ├── db/              # Database connection
│   ├── entities/        # SeaORM entity models
│   ├── main.rs          # Server entry point
│   └── cli/             # CLI tools (masterctl)
├── migration/           # Database migrations
├── Dockerfile           # Multi-stage build
├── docker-compose.yml   # Development stack
└── docs/                # Specifications and plans
```

## Next Steps

To continue implementation, see [docs/implementation_plan.md](docs/implementation_plan.md) for the complete phase breakdown.

## License

See root project LICENSE.
