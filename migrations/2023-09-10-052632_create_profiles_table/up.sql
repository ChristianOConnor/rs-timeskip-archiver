-- Your SQL goes here
CREATE TABLE profiles (
    id INTEGER NOT NULL PRIMARY KEY,
    profile_name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);