{
  buildNpmPackage,
  lib,
  version ? null,
}:
buildNpmPackage {
  name = "wcpc-frontend";
  inherit version;
  src = ../frontend;
  packageJSON = ../frontend/package.json;

  npmDepsHash = "sha256-Qobi9kHg86PTz4R/ALKrPA0QEu2SYCa42hmG8vBGMMY=";

  installPhase = "cp -r dist/ $out";

  meta = {
    description = "Frontend to WCPC";
  };
}
