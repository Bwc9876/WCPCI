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

  npmDepsHash = "sha256-HdqEKDbbnUnfpLiYkXQZ4oRBnBzR8v1ozeb+NDaQnqA=";

  installPhase = ''
    cp -r dist/ $out
  '';

  meta = {
    description = "Frontend to WCPC";
  };
}
