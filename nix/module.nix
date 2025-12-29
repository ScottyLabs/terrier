{ self }:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.terrier;
in
{
  options.services.terrier = {
    enable = lib.mkEnableOption "Terrier";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      description = "The Terrier package to use";
    };

    environmentFile = lib.mkOption {
      type = lib.types.path;
      description = "Path to environment file containing secrets";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "terrier";
      description = "User to run Terrier as";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "terrier";
      description = "Group to run Terrier as";
    };

    extraGroups = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "Extra groups for the terrier user (e.g., for socket access)";
      example = [ "redis-terrier" ];
    };

    dependencies = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "Systemd services that Terrier depends on";
      example = [
        "postgresql.service"
        "redis.service"
        "minio.service"
      ];
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      extraGroups = cfg.extraGroups;
    };
    users.groups.${cfg.group} = { };

    services.postgresql = {
      ensureUsers = lib.mkIf config.services.postgresql.enable [
        {
          name = cfg.user;
          ensureDBOwnership = true;
        }
      ];
      ensureDatabases = lib.mkIf config.services.postgresql.enable [
        "terrier"
      ];
    };

    systemd.services.terrier = {
      description = "Terrier";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ] ++ cfg.dependencies;
      requires = cfg.dependencies;

      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        WorkingDirectory = "${cfg.package}";
        EnvironmentFile = cfg.environmentFile;
        ExecStart = "${cfg.package}/terrier";
        Restart = "on-failure";
        RestartSec = "10s";

        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
      };
    };
  };
}
