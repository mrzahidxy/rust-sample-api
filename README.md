# Rust project management API

A reusable Axum, SQLx, and PostgreSQL backend template with email/password authentication, ADMIN/USER roles, and a practical projects/tasks example. Each user can access only their own projects and tasks.

## Generate and run

```sh
cargo install cargo-generate --locked
cargo generate --git https://github.com/mrzahidxy/rust-sample-api.git --name my-service
cd my-service
cp .env.example .env
openssl rand -hex 32
```

Set `JWT_SECRET` in `.env`, then start PostgreSQL and the API:

```sh
docker compose up -d --wait
cargo migrate
SEED_EMAIL=admin@example.com SEED_PASSWORD=a-strong-development-password SEED_ROLE=ADMIN cargo seed
cargo run
```

The API listens at `http://127.0.0.1:3000`. API docs are at `/docs`; OpenAPI is at `/openapi.json`.

## API

- `POST /api/auth/register` and `/login`: register or log in; registration always creates USER.
- `POST /api/auth/refresh`: rotate the refresh token.
- `POST /api/auth/logout`: revoke the current session.
- `GET /api/auth/me`: current profile.
- `/api/users`: ADMIN user management; users can read/update their own profile.
- `/api/projects`: create, list, read, update, and delete your projects.
- `/api/projects/:id/tasks`: create and list tasks for your project.
- `/api/projects/:id/tasks/:task`: update or delete a task.

Tasks have a title, description, due date, priority 1–5, and status `TODO`, `IN_PROGRESS`, `BLOCKED`, or `DONE`. Descriptions are limited to 500 bytes; due dates must be valid `YYYY-MM-DD` dates. PATCH preserves omitted fields; send `null` to clear a project description or a task description/due date. All project and task queries enforce ownership in SQL. Lists accept `page` and `limit` query parameters.

Passwords use Argon2id. Access tokens are short lived; refresh credentials are stored hashed in PostgreSQL and rotated atomically. ADMIN and USER roles are checked from the database on each authenticated request.

## Configuration

See `.env.example` for database, JWT, listener, connection pool, logging, CORS, and application metadata settings. Generate a fresh random JWT secret before use. `SEED_ROLE` accepts only ADMIN or USER; seeding an existing email never changes its role or password.

The built-in rate limiter is in-memory, counts the socket peer address, and is intended for a single service instance. For multiple instances or a reverse proxy, configure a trusted ingress with shared limits and set `APP_RATE_LIMIT_ENABLED=false` so all clients do not share one application-process quota.

## Database migrations

This template starts with a clean migration history and is intended for a fresh database. Do not point it at a database initialized by an earlier version of `rust-sample-api`: the replaced migration history may cause SQLx to reject the database. For an existing installation, keep its migration history and prepare a separate migration plan before upgrading.

## Project layout

`routes/` handles HTTP, `services/` owns application rules and database queries, `middleware/` handles authentication and request limits, and `migrations/` contains the schema. Replace the example project/task modules with your own domain while retaining the auth and user modules.

## Optional checks

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
DATABASE_URL=postgres://app:app@127.0.0.1:5432/app cargo test --locked --all-targets
```

The Docker image runs as a non-root user. Local Compose supplies PostgreSQL. No frontend, tenant model, bookings, analytics, payment, or upload integration is included.
