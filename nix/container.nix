{
  dockerTools,
  wrapper,
  stream ? false,
  # FIXME: REMOVE THIS, only for debugging
  pkgs,
}:
dockerTools
.${
  if stream
  then "streamLayeredImage"
  else "buildLayeredImage"
} {
  # TODO(Spoon): optimize layers?
  name = "wcpc";
  tag = "latest";
  maxLayers = 125;
  contents = [wrapper dockerTools.caCertificates /*pkgs.coreutils pkgs.bashInteractive pkgs.nano*/];
  config = {
    Cmd = ["wcpc"];
    ExposedPorts."443/tcp" = {};
    Env = [
      "PATH=/bin"
      "ROCKET_SAML={certs=\"/secrets/saml_cert.pem\",key=\"/secrets/saml_key.pem\"}"
      "ROCKET_TLS={certs=\"/secrets/tls_cert.pem\",key=\"/secrets/tls_key.pem\"}"
      "ROCKET_DATABASES={sqlite_db={url=\"/database/database.sqlite\"}}"
    ];
    Volumes."/secrets" = {};
    Volumes."/database" = {};
    WorkingDir = "/secrets"; # To load .env
  };
}
