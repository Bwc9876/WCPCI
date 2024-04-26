{
  rustPlatform,
  stdenv,
  lib,
  libiconv,
  libtool,
  libclang,
  libxml2,
  libxslt,
  llvmPackages,
  buildNpmPackage,
  openssl,
  callPackage,
  makeWrapper,
  sqlx-cli,
  pkg-config,
  xmlsec,
  ...
}:
rustPlatform.buildRustPackage rec {
  name = "wcpc";
  version = "0.0.1";
  src = with lib.fileset;
    toSource {
      root = ../.;
      fileset = unions [
        ../.cargo
        ../.env
        ../src
        ../frontend
        ../public
        ../Cargo.toml
        ../migrations
        ../Cargo.lock
        ../Rocket.toml
      ];
    };

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  BINDGEN_EXTRA_CLANG_ARGS = "${builtins.readFile "${stdenv.cc}/nix-support/libc-crt1-cflags"} \
      ${builtins.readFile "${stdenv.cc}/nix-support/libc-cflags"} \
      ${builtins.readFile "${stdenv.cc}/nix-support/cc-cflags"} \
      ${builtins.readFile "${stdenv.cc}/nix-support/libcxx-cxxflags"} \
      -idirafter ${libiconv}/include \
      ${lib.optionalString stdenv.cc.isClang "-idirafter ${stdenv.cc.cc}/lib/clang/${lib.getVersion stdenv.cc.cc}/include"} \
      ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config} -idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${lib.getVersion stdenv.cc.cc}/include"} \
  ";

  nativeBuildInputs = [
    libiconv
    libtool
    libxml2
    libxslt
    llvmPackages.libclang
    openssl
    pkg-config
    xmlsec
    makeWrapper
    sqlx-cli
  ];

  doCheck = false; # TODO: Remove if when we get tests

  buildInputs = [
    libiconv
    libtool
    libxml2
    libxslt
    llvmPackages.libclang
    openssl
    pkg-config
    xmlsec
  ];

  preBuild = ''
    sqlx database setup
  '';

  postInstall = let
    frontend = import ./frontend.nix {inherit lib buildNpmPackage;};
  in ''
    wrapProgram $out/bin/${meta.mainProgram} --set ROCKET_TEMPLATE_DIR ${frontend} --set ROCKET_PUBLIC_DIR ${../public}
  '';

  meta = with lib; {
    description = "An OJ system";
    mainProgram = "wcpc";
  };
}
