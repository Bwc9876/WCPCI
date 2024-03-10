{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  name = "wcpc-shell";
  buildInputs = with pkgs; [
    rustc
    cargo
    clippy
    rustfmt
    nodejs
    nodePackages.pnpm
    gcc
    sqlx-cli
    just
    mprocs
  ];
  shellHook = '''';
}
