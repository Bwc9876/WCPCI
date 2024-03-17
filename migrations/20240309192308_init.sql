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
    name VARCHAR(100) NOT NULL,
    description TEXT NOT NULL,
    cpu_time INTEGER NOT NULL
);

CREATE TABLE test_case (
    id INTEGER PRIMARY KEY NOT NULL,
    problem_id INTEGER NOT NULL,
    ord INTEGER NOT NULL,
    stdin TEXT NOT NULL,
    expected_pattern TEXT NOT NULL,
    use_regex BOOLEAN NOT NULL,
    case_insensitive BOOLEAN NOT NULL,
    FOREIGN KEY (problem_id) REFERENCES problem(id) ON DELETE CASCADE
    UNIQUE (problem_id, ord)
);

CREATE TABLE judge_run (
    id INTEGER PRIMARY KEY NOT NULL,
    problem_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    amount_run INTEGER NOT NULL,
    total_cases INTEGER NOT NULL,
    error TEXT,
    ran_at TIMESTAMP NOT NULL
);

