setup:
    cd frontend && pnpm i
    cargo sqlx database setup
    -cp -n .dev.env.template .dev.env

dev:
    mprocs "cargo run" "cd frontend && pnpm watch"

format:
    cargo fmt
    cd frontend && pnpm format

lint:
    cargo lint

update:
    cargo update
    cd frontend && pnpm update --latest

