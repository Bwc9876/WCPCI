
# WCPC WIP Thing

This is a work in progress thing for the WCPC. It's a thing. It's a work in progress. It's a thing that's a work in progress

## Dev Setup

1. `nix develop`
2. `just setup`
3. Fill `.dev.env` file with the correct values for your setup

Now run `just-dev` to build the frontend and start the backend.

If you need to auto-rebuild the frontend on changes too, run `just dev-watch`.

Once either are run, connect to `http://localhost:8000` to see the frontend

### OAuth No Workies

Make sure you're doing `localhost:8000` and not `127.0.0.1:8000`.
Callback URLs are pointing to `localhost` and not `127.0.0.1`
so CORS will kick in and block state cookie access.

## Production Setup

--TODO
