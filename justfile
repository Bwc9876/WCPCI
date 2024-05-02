_default:
    @just --list --unsorted --justfile {{justfile()}}


# Set up frontend, database, and environment variables
setup:
    cd frontend && npm i
    cargo sqlx database setup
    -cp -n .env.template .dev.env

# Start a development server
dev:
    cd frontend && npm run build
    cargo run

# Run the backend and recompile the frontend when the frontend changes
dev-watch:
    mprocs "cargo run" "cd frontend && npm run watch"

# Format backend & frontend
format:
    cargo fmt
    cd frontend && npm run format
    nix fmt

# Lint the backend
lint:
    cargo lint

# Update frontend & backend dependencies
update:
    cargo update
    cd frontend && npm update --latest
    nix flake update
