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

  npmDepsHash = "sha256-kPMiIHR4TFJ3vWa0wHVSFrjCuKfRvDfVUMptALmMNds=";

  installPhase = ''
    cp -r dist/ $out
  '';

  meta = {
    description = "Frontend to WCPC";
  };
}
