-- NOTE: This file **WILL** change during dev, meaning you will have to run `cargo sqlx database reset -y` sometimes when pulling
-- Migrations will only be used properly later in development, for now, we will just use them to create the initial schema

CREATE TABLE IF NOT EXISTS user (
    id INTEGER PRIMARY KEY NOT NULL,
    email TEXT NOT NULL UNIQUE,
    bio TEXT NOT NULL DEFAULT '',
    default_display_name VARCHAR(100) NOT NULL,
    display_name VARCHAR(32),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS session (
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL,
    token TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS contest (
    id INTEGER PRIMARY KEY NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP NOT NULL,
    registration_deadline TIMESTAMP NOT NULL,
    max_participants INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS participant (
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL,
    contest_id INTEGER NOT NULL,
    registered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES user(id) ON DELETE CASCADE,
    FOREIGN KEY (contest_id) REFERENCES contest(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS problem (
    id INTEGER PRIMARY KEY NOT NULL,
    contest_id INTEGER NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    cpu_time INTEGER,
    FOREIGN KEY (contest_id) REFERENCES contest(id) ON DELETE CASCADE
);
