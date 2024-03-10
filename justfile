setup:
    cd frontend && pnpm i && cd ..
    cargo sqlx database setup
    -cp -n .dev.env.template .dev.env

dev-watch:
    mprocs "cargo run" "cd frontend && pnpm watch"

dev:
    cd frontend && pnpm build && cd ..
    cargo run

format:
    cargo fmt
    cd frontend && pnpm format && cd ..

lint:
    cargo lint

update:
    cargo update
    cd frontend && pnpm update --latest && cd ..

