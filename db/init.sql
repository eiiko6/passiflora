-- Users table
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    email TEXT UNIQUE,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
);

-- Items table linked to users
CREATE TABLE IF NOT EXISTS files (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    mime_type TEXT NOT NULL
);

