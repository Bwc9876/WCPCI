{
  description = "Flake for wcpc web interface";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable"; # TODO(Spoon): Do we want to track stable?

  outputs = {
    self,
    nixpkgs,
  }: let
    forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
    pkgsFor = system:
      (import nixpkgs) {
        inherit system;
      };

    gitRev = self.shortRev or self.dirtyShortRev or "";
    # TODO: Change back once Spoon fixes this
    rawVersion = "0.1.0"; # (builtins.importTOML ./Cargo.toml).version;
    version = rawVersion + "-" + gitRev;
    packages = pkgs: rec {
      backend = pkgs.callPackage ./nix/backend.nix {inherit version gitRev;};
      frontend = pkgs.callPackage ./nix/frontend.nix {inherit version;};
      wrapper = pkgs.callPackage ./nix/wrapper.nix {inherit version backend frontend rocket_config;};
      rocket_config = pkgs.callPackage ./nix/rocket_config.nix {openjdk = pkgs.jre_minimal.override {modules = ["java.base" "jdk.compiler"];};};

      container = pkgs.callPackage ./nix/container.nix {inherit wrapper;};
      container-stream = pkgs.runCommand "container-stream" {
        script = container.override {stream = true;};
        nativeBuildInputs = [pkgs.coreutils];
      } "mkdir -p $out/bin; ln -s $script $out/bin/container-stream";
      nixos-vm = (pkgs.nixos [{environment.systemPackages = [default];} ./nix/staging-nixos-config.nix]).vm;

      default = wrapper;
    };
  in {
    packages = forAllSystems (system: packages (pkgsFor system));
    formatter = forAllSystems (system: (pkgsFor system).alejandra);
    devShells = let
      shellPackages = pkgs: (with pkgs; [just mprocs rustfmt cargo clippy rust-analyzer]);
    in
      forAllSystems (system: {
        default =
          (packages (pkgsFor system)).backend.overrideAttrs
          (
            old: let
              pkgs = pkgsFor system;
            in {
              # Needed for rust-analyzer to find openssl and libxml2
              # Also prevents needless rebuilding of their -sys crates
              OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
              OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
              LIBXML2 = "${pkgs.libxml2.out}/lib/libxml2.so";
              nativeBuildInputs = old.nativeBuildInputs ++ shellPackages pkgs;
            }
          );
      });
  };
}
/*
Put .env in volume, cd into volume


Considerations for container:
How will the certs (TLS, SAML) be renewed?




docs:

`nix run .#container-stream | docker load
*/

