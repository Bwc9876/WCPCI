{
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

  networking.firewall.enable = false;

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
        basicAuth.tester = "WCPC_T3ST1NG!";
      };
    };
  };

  programs.neovim = {
    enable = true;
    defaultEditor = true;
    viAlias = true;
    vimAlias = true;
  };
}
