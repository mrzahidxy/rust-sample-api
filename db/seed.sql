-- Seed data for the Todo API.
-- Run this after the app has started once, so the `todos` table already exists.

DELETE FROM todos;

INSERT INTO todos (id, title, completed) VALUES
    ('11111111-1111-1111-1111-111111111111', 'Learn Rust basics', false),
    ('22222222-2222-2222-2222-222222222222', 'Build an Axum API', true),
    ('33333333-3333-3333-3333-333333333333', 'Practice PostgreSQL queries', false);
