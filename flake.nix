{
  description = "Flake for wcpc web interface";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    nix-std.url = "github:chessai/nix-std";
  };

  outputs = {
    self,
    nixpkgs,
    nix-std,
  }: let
    forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
    pkgsFor = system:
      (import nixpkgs) {
        inherit system;
      };
  in rec {
    packages = forAllSystems (system: {
      default = (pkgsFor system).callPackage ./nix/wcpc.nix {pkgs = pkgsFor system;};
      frontend = (pkgsFor system).callPackage ./nix/frontend.nix {pkgs = pkgsFor system;};
    });
    nixosConfigurations.wcpc-staging = nixpkgs.lib.nixosSystem rec {
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        lib = nixpkgs.lib;
      };
      specialArgs = {
        wcpc = packages.${system}.default;
        std = nix-std.lib;
      };
      modules = [
        ./nix/staging.nix
      ];
    };
    formatter = forAllSystems (system: (pkgsFor system).alejandra);
    devShells = forAllSystems (system: {default = import ./nix/shell.nix {pkgs = pkgsFor system;};});
  };
}
