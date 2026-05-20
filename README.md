# Rust Todo API

Minimal todo REST API built with Rust, Axum, Tokio, SQLx, and PostgreSQL.

## Tech Stack
- Rust 2021
- Axum
- Tokio
- SQLx
- PostgreSQL
- Serde
- UUID

## Setup
1. Create a `.env` file with `DATABASE_URL=your_postgres_connection_string`.
2. Build the project:
   ```bash
   cargo build
   ```
3. Seed sample data:
   ```bash
   cargo seed
   ```
4. Start the API at `http://127.0.0.1:3000`:
   ```bash
   cargo run
   ```

## API Endpoints
- `GET /health` -> health check
- `GET /todos` -> list todos
- `POST /todos` -> create todo with JSON body `{ "title": "Learn Rust" }`
- `GET /todos/:id` -> fetch one todo
- `PATCH /todos/:id` -> update fields with JSON body like `{ "title": "Updated", "completed": true }`
- `DELETE /todos/:id` -> delete one todo

## Quick Checks
```bash
curl http://127.0.0.1:3000/health
curl http://127.0.0.1:3000/todos
curl http://127.0.0.1:3000/todos/11111111-1111-1111-1111-111111111111
```

## License
No license is currently specified for this repository.
