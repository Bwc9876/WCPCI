{
  pkgs,
  wcpc,
  std,
  ...
}: let
  rocketConfig = std.serde.toTOML {
    release = {
      cli_colors = false;
      timezone = "America/New_York";
      admins = ["bwc9876@example.com"];
      url = "http://127.0.0.1:8000";
      port = 8000;
      ip_header = "X-Forwarded-For";
      address = "127.0.0.1";

      saml = {
        entity_id = "urn:wcpc:saml-debugging";
        idp_meta_url = "https://mocksaml.com/api/namespace/wcpc_testing_provider/saml/metadata";
        contact_person = "Ben";
        contact_email = "bwc9876@example.org";
        contact_telephone = "555-555-5555";
        organization = "WCPC";

        attrs = {
          display_name = "firstName";
          email = "email";
        };
      };

      oauth = {
        github = {
          provider = "GitHub";
          redirect_uri = "http://localhost:8000/auth/github/callback";
        };
        google = {
          provider = "Google";
          redirect_uri = "http://localhost:8000/auth/google/callback";
        };
      };

      databases.sqlite_db = {
        url = "/home/wcpc/database.sqlite";
      };

      run = {
        max_program_length = 100000;
        default_language = "python";

        languages = {
          python = {
            name = "Python";
            default_code = ''
              print("Hello, World!")
            '';
            tabler_icon = "brand-python";
            monaco_contribution = "python";
            file_name = "main.py";
            compile_cmd = "";
            run_cmd = "python main.py";
          };
        };
      };
    };
  };
in {
  system.stateVersion = "23.05";

  time.timeZone = "America/New_York";

  users = {
    users.wcpc = {
      isNormalUser = true;
      description = "WCPC User";
      password = "wcpcpass";
      createHome = true;
      extraGroups = ["wheel"];
    };
  };

  networking.firewall = {
    enable = false;
    allowedTCPPorts = [8000 8001];
  };

  services.nginx = {
    enable = true;
    recommendedOptimisation = true;
    virtualHosts.wcpc = {
      listen = [
        {
          port = 80;
          addr = "0.0.0.0";
        }
      ];
      default = true;
      locations."/" = {
        recommendedProxySettings = true;
        proxyPass = "http://127.0.0.1:8000";
        proxyWebsockets = true;
        basicAuth = {
          tester = "WCPC_T3ST1NG!";
        };
      };
    };
  };

  environment.variables = {
    ROCKET_CONFIG = pkgs.writeText "rocket.toml" rocketConfig;
  };

  environment.systemPackages = with pkgs; [
    wcpc
    python3
  ];
}
