{
  dockerTools,
  wrapper,
  stream ? false,

  # FIXME: REMOVE THIS, only for debugging
  pkgs,

}:
dockerTools.${if stream then "streamLayeredImage" else "buildLayeredImage"} {
  name = "wcpc";
  tag = "latest";
  contents = [wrapper dockerTools.caCertificates pkgs.coreutils pkgs.bashInteractive pkgs.nano];
  config = {
    Cmd = ["wcpc"];
    #ExposedPorts."443/tcp" = {};
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

/*
TODO(Spoon):
Healthcheck?

port 80? - redirect (& acme challenge?)





secrets volume needs:
{saml,tls}_{cert,key}.pem
.env
*/