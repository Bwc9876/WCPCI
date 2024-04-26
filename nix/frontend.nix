{
  buildNpmPackage,
  lib,
  ...
}:
buildNpmPackage {
  name = "wcpc-frontend";
  src = ../frontend;
  packageJSON = ../frontend/package.json;

  npmDepsHash = "sha256-AO9O/j7Ff95cNOKUlKUJrzyPOX7RoFz0TND3tY67LZw=";

  distPhase = "true";
  dontInstall = true;
  installInPlace = true;
  distDir = "../frontend/dist";

  postBuild = ''
    cp -r dist/ $out
  '';

  meta = with lib; {
    description = "Frontend to WCPC";
  };
}
