-- Add up migration script here
CREATE TABLE IF NOT EXISTS users (
 id BIGSERIAL PRIMARY KEY NOT NULL,
 username TEXT NOT NULL UNIQUE,
 password_hash TEXT NOT NULL
);