{
  buildNpmPackage,
  lib,
  ...
}:
buildNpmPackage {
  name = "wcpc-frontend";
  src = ../frontend;
  packageJSON = ../frontend/package.json;

  npmDepsHash = "sha256-xRrhLfGY/PO3mnjgwdECJbUDZYHGB1VPoMfdMeHkKzk=";

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
