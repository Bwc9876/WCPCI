{
  python3,
  rustc,
  ghc,
  typescript,
  nodejs,
  openjdk,
  mono,
  writers,
}:
writers.writeTOML "rocket.toml" {
  release = {
    # TODO(Spoon): rework this
    cli_colors = false;
    timezone = "America/New_York";
    port = 443;
    # ip_header = "X-Forwarded-For";
    address = "0.0.0.0";
    url = "https://codingcomp.cs.wcupa.edu";

    # TODO(Spoon): Do the data things
    saml = {
      entity_id = "urn:wcu:sp:wcpc";
      idp_meta_url = "https://mocksaml.com/api/namespace/wcpc_testing_provider/saml/metadata";
      contact_person = "Ben";
      contact_email = "bwc9876@example.org";
      contact_telephone = "555-555-5555";
      organization = "West Chester University Computer Science Department";

      attrs = {
        display_name = "firstName";
        email = "email";
      };
    };

    databases.sqlite_db.url = "database.sqlite"; # When running in a container, this will be overridden

    oauth = {
      github.provider = "GitHub";
      google.provider = "Google";
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
          compile_cmd = ""; # TODO(Spoon): do validation here
          run_cmd = "${python3}/bin/python main.py";
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
          compile_cmd = "${rustc}/bin/rustc main.rs -o main";
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
          compile_cmd = "${ghc}/bin/ghc main.hs";
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
          compile_cmd = "${typescript}/bin/tsc index.ts";
          run_cmd = "${nodejs}/bin/node index.js";
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
          compile_cmd = "${openjdk}/bin/javac Main.java";
          run_cmd = "${openjdk}/bin/java Main";
        };
        csharp = {
          name = "C#";
          default_code = ''
            public class Program {
                public static void Main(string[] args) {
                    System.Console.WriteLine("Hello, World!");
                }
            }
          '';
          tabler_icon = "brand-c-sharp";
          monaco_contribution = "csharp";
          file_name = "Program.cs";
          compile_cmd = "${mono}/bin/mcs Program.cs";
          run_cmd = "${mono}/bin/mono Program.exe";
        };
      };
    };
  };
}
