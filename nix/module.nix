{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.programs.ditox;
  tomlFormat = pkgs.formats.toml { };
in {
  options.programs.ditox = {
    enable = mkEnableOption "ditox clipboard manager";

    package = mkOption {
      type = types.package;
      default = pkgs.ditox;
      defaultText = literalExpression "pkgs.ditox";
      description = "The ditox package to use.";
    };

    settings = mkOption {
      type = tomlFormat.type;
      default = { };
      example = literalExpression ''
        {
          general = {
            max_entries = 1000;
            poll_interval_ms = 300;
          };
          ui = {
            show_preview = true;
            date_format = "relative";
          };
        }
      '';
      description = "Configuration for ditox, written to config.toml";
    };

    systemd = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable systemd user service for clipboard watching.";
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."ditox/config.toml" = mkIf (cfg.settings != { }) {
      source = tomlFormat.generate "ditox-config" cfg.settings;
    };

    systemd.user.services.ditox = mkIf cfg.systemd.enable {
      Unit = {
        Description = "Ditox clipboard watcher";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };

      Service = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/ditox watch";
        Restart = "on-failure";
        RestartSec = 5;
      };

      Install = {
        WantedBy = [ "graphical-session.target" ];
      };
    };
  };
}
