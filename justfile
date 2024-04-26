setup:
    cd frontend && pnpm i
    cargo sqlx database setup
    -cp -n .dev.env.template .dev.env

dev-watch:
    mprocs "cargo run" "cd frontend && pnpm watch"

dev:
    cd frontend && npm run build
    cargo run

format:
    cargo fmt
    cd frontend && npm run format
    nix fmt

lint:
    cargo lint

update:
    cargo update
    cd frontend && npm update --latest

