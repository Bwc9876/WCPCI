{
  description = "Flake for WCPC deployment";

  inputs.wcpc.url = "github:Bwc9876/WCPCI";

  outputs = {wcpc, ...}: let
    nixpkgs = wcpc.inputs.nixpkgs;
    forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
    pkgsFor = system: import nixpkgs {inherit system;};

    packages = system: let
      pkgs = pkgsFor system;
      packages = wcpc.packages.${system};
    in rec {
      rocket_config = pkgs.callPackage ./rocket_config.nix {
        openjdk = pkgs.jre_minimal.override {modules = ["java.base" "jdk.compiler"];}; # This builds a JDK with only these Java Platform Modules
      };
      wrapper = packages.wrapper.override {inherit rocket_config;};

      container = packages.container.override {inherit wrapper;};
      container-stream = packages.container-stream.override {script = container.override {stream = true;};};

      default = container-stream;
    };
  in {
    packages = forAllSystems packages;
  };
}
