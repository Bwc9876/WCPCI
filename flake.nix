{
  description = "Flake for wcpc web interface";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    # pnpm2nix = {
    #   url = "github:nzbr/pnpm2nix-nzbr";
    #   inputs.nixpkgs.follows = "nixpkgs";
    # };
  };

  outputs = {
    self,
    nixpkgs,
    # pnpm2nix,
  }: let
    forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
    pkgsFor = system:
      (import nixpkgs) {
        inherit system;
      };
  in {
    formatter = forAllSystems (system: (pkgsFor system).alejandra);
    devShells = forAllSystems (system: {default = import ./nix/shell.nix {pkgs = pkgsFor system;};});
  };
}
