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
  openssl,
  sqlx-cli,
  pkg-config,
  xmlsec,
  version ? null,
  gitRev ? "",
}:
rustPlatform.buildRustPackage rec {
  name = "wcpc";
  inherit version;
  GIT_COMMIT_HASH = gitRev;

  src = with lib.fileset;
    toSource {
      root = ../.;
      fileset = unions [
        ../src
        ../Cargo.toml
        ../migrations
        ../Cargo.lock
      ];
    };

  cargoLock.lockFile = ../Cargo.lock;

  # TODO(Spoon): clean this up, no ifd?

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  BINDGEN_EXTRA_CLANG_ARGS = "${builtins.readFile "${stdenv.cc}/nix-support/libc-crt1-cflags"} \
      ${builtins.readFile "${stdenv.cc}/nix-support/libc-cflags"} \
      ${builtins.readFile "${stdenv.cc}/nix-support/cc-cflags"} \
      ${builtins.readFile "${stdenv.cc}/nix-support/libcxx-cxxflags"} \
      -idirafter ${libiconv}/include \
      ${lib.optionalString stdenv.cc.isClang "-idirafter ${stdenv.cc.cc}/lib/clang/${lib.getVersion stdenv.cc.cc}/include"} \
      ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config} -idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${lib.getVersion stdenv.cc.cc}/include"} \
  ";

  # TODO(Spoon): clean up deps
  nativeBuildInputs = [
    pkg-config
    xmlsec
    sqlx-cli
  ];

  doCheck = false; # TODO: Remove if/when we get tests

  buildInputs = [
    libiconv
    libtool
    libxml2
    libxslt
    # llvmPackages.libclang
    openssl
    xmlsec
  ];

  # SQLx needs a database to check against
  preBuild = "sqlx database setup";
  DATABASE_URL = "sqlite://database.sqlite";

  meta = {
    description = "WCPC Backend";
    mainProgram = "wcpc";
  };
}
