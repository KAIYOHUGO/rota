{ rota }:
{
  pkgs,
  config,
  option,
  lib,
  ...
}:
let
  mkConfig =
    text:
    pkgs.writeTextFile {
      name = "rota-config.kdl";
      text = text;
    };
  cfg = config.services.rota;
in
{
  options.services.rota = {
    enable = lib.mkEnableOption "Enable rota service";
    debug = lib.mkEnableOption "Enable debug log";

    config = lib.mkOption {
      type = lib.types.lines;
    };
    configFile = lib.mkOption {
      type = lib.types.path;
      default = mkConfig cfg.config;
    };
    packages = lib.mkOption {
      type = lib.types.listOf lib.types.package;
      default = [ ];
    };
    enviroment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.rota = {
      wantedBy = [ "graphical.target" ];
      after = [
        "iio-sensor-proxy.service"
        "graphical.target"
      ];
      environment = lib.mkMerge [
        (lib.mkIf cfg.debug {
          RUST_LOG = "debug";
        })
        cfg.enviroment
      ];

      path = cfg.packages;
      serviceConfig = {
        ExecStart = "${rota}/bin/rota ${cfg.configFile}";
        Restart = "always";
      };

    };

  };
}
