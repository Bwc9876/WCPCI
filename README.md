
# WCPC WIP Thing

This is a work in progress thing for the WCPC. It's a thing. It's a work in progress. It's a thing that's a work in progress

## Dev Setup

1. `nix develop`
2. `just setup`
3. Fill `.dev.env` file with the correct values for your setup
4. `just dev`

This will start mprocs with two commands running:

- `pnpm watch`: Will watch for changes in the frontend and rebuild the frontend on changes
- `cargo run`: Will start the backend server

Once run connect to `http://localhost:8000` to see the frontend

## Production Setup

--TODO
