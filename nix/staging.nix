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
      port = 8000;
      ip_header = "X-Forwarded-For";
      address = "0.0.0.0";

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
        };
        google = {
          provider = "Google";
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
          rust = {
            name = "Rust";
            default_code = ''
              fn main() {
                  println!("Hello, World!");
              }
            '';
            tabler_icon = "brand-rust";
            monaco_contribution = "rust";
            file_name = "main.rs";
            compile_cmd = "rustc main.rs -o main";
            run_cmd = "./main";
          };
          haskell = {
            name = "Haskell";
            default_code = ''
              main = putStrLn "Hello, World!"
            '';
            tabler_icon = "lambda";
            monaco_contribution = "haskell";
            file_name = "main.hs";
            compile_cmd = "ghc main.hs";
            run_cmd = "./main";
          };
          typescript = {
            name = "TypeScript / JavaScript";
            default_code = ''
              console.log("Hello, World!");
            '';
            tabler_icon = "brand-typescript";
            monaco_contribution = "typescript";
            file_name = "index.ts";
            compile_cmd = "tsc index.ts";
            run_cmd = "node index.js";
          };
          java = {
            name = "Java";
            default_code = ''
              public class Main {
                  public static void main(String[] args) {
                      System.out.println("Hello, World!");
                  }
              }
            '';
            tabler_icon = "coffee";
            monaco_contribution = "java";
            file_name = "Main.java";
            compile_cmd = "javac Main.java";
            run_cmd = "java Main";
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

  programs.neovim = {
    enable = true;
    defaultEditor = true;
    viAlias = true;
    vimAlias = true;
  };

  environment.systemPackages = with pkgs; [
    wcpc
    python3
    gcc
    typescript
    nodejs
    jdk
    rustc
    ghc
    cargo
  ];
}
