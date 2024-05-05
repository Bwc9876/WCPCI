
# WCPC WIP Thing

This is a work in progress thing for the WCPC. It's a thing. It's a work in progress. It's a thing that's a work in progress

## Dev Setup

1. `nix develop`
2. `just setup`
3. Fill `.dev.env` file with the correct values for your setup

Now run `just dev` to build the frontend and start the backend.

If you need to auto-rebuild the frontend on changes too, run `just dev-watch`.

Once either are run, connect to `http://localhost:8000` to see the frontend

### OAuth No Workies

Make sure you're doing `localhost:8000` and not `127.0.0.1:8000`.
Callback URLs are pointing to `localhost` and not `127.0.0.1`
so CORS will kick in and block state cookie access.

## Production Setup

We recommend deploying with Nix. Docker makes it easy to build on a machine that has Nix, and deploy on a machine that doesn't.


### Nix + Docker

In an empty directory, either locally or on the server you will deploy to:

```sh
nix flake init -t github:Bwc9876/WCPCI
```
Then follow the directions in the new `README.md`. (Also in this repo under `nix-template/`)

### Nix without Docker

You can also use just Nix and deploy as a systemd service or something else.

To deploy as a systemd service with NixOS, make a `rocket_config.nix` (based on [`nix-template/rocket_config.nix`](/nix-template/rocket_config.nix)) and `.env` for sensitive options. The options are documented [in the deployment guide](/DEPLOYMENT.md#configuration). Add the following to your config:
```nix
systemd.services.wcpc = let
  rocket_config = pkgs.callPackage <your rocket_config.nix> {};
  wcpcDrv = <wcpc flake>.packages.${pkgs.system}.wrapper.override {inherit rocket_config;};
in {
  wants = ["network.target"];
  wantedBy = ["multi-user.target"];
  serviceConfig = {
    ExecStart = "${lib.getExe wcpcDrv}";
    EnvironmentFile = "/path/to/.env"; # This doesn't need to be named `.env`
  };
  env = {
    # These are read-only
    ROCKET_SAML='{certs="/path/to/saml_cert.pem",key="/path/to/saml_key.pem"}';
    ROCKET_TLS='{certs="/path/to/tls_cert.pem",key="/path/to/tls_key.pem"}';

    # This is read-write
    ROCKET_DATABASES='{sqlite_db={url="/path/to/database.sqlite"}}';
  };
};

```
This has not been tested yet. Sorry.
<!-- TODO(Spoon): test this -->

### Without Nix

See [the deployment guide](/DEPLOYMENT.md)
