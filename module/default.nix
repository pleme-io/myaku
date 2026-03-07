# Myaku home-manager module — GPU system monitor with typed config + daemon
#
# Namespace: blackmatter.components.myaku.*
#
# Generates YAML config from typed Nix options, loaded by shikumi at runtime.
# Supports hot-reload via symlink-aware file watching.
#
# Module factory: receives { hmHelpers } from flake.nix, returns HM module.
{ hmHelpers }:
{
  lib,
  config,
  pkgs,
  ...
}:
with lib;
let
  inherit (hmHelpers) mkLaunchdService mkSystemdService;
  cfg = config.blackmatter.components.myaku;
  isDarwin = pkgs.stdenv.isDarwin;

  logDir =
    if isDarwin then "${config.home.homeDirectory}/Library/Logs"
    else "${config.home.homeDirectory}/.local/share/myaku/logs";

  # -- YAML config generation --------------------------------------------------
  settingsAttr = let
    appearance = filterAttrs (_: v: v != null) {
      inherit (cfg.appearance) width height font_size opacity refresh_rate_ms;
    };

    monitoring = filterAttrs (_: v: v != null) {
      inherit (cfg.monitoring) show_cpu show_memory show_disk show_network show_gpu;
    };

    processes = filterAttrs (_: v: v != null) {
      inherit (cfg.processes) sort_by show_threads auto_refresh;
    };

    alerts = filterAttrs (_: v: v != null) {
      inherit (cfg.alerts) cpu_threshold memory_threshold disk_threshold;
    };

    daemon = optionalAttrs cfg.daemon.enable (filterAttrs (_: v: v != null) {
      enable = cfg.daemon.enable;
      metrics_port = cfg.daemon.metrics_port;
      history_retention_hours = cfg.daemon.history_retention_hours;
    });
  in
    filterAttrs (_: v: v != {} && v != null) {
      inherit appearance monitoring processes alerts daemon;
    }
    // cfg.extraSettings;

  yamlConfig = pkgs.writeText "myaku.yaml"
    (lib.generators.toYAML { } settingsAttr);
in
{
  options.blackmatter.components.myaku = {
    enable = mkEnableOption "Myaku — GPU system monitor";

    package = mkOption {
      type = types.package;
      default = pkgs.myaku;
      description = "The myaku package to use.";
    };

    # -- Appearance ------------------------------------------------------------
    appearance = {
      width = mkOption {
        type = types.int;
        default = 800;
        description = "Window width in pixels.";
      };

      height = mkOption {
        type = types.int;
        default = 600;
        description = "Window height in pixels.";
      };

      font_size = mkOption {
        type = types.float;
        default = 13.0;
        description = "Font size in points.";
      };

      opacity = mkOption {
        type = types.float;
        default = 0.95;
        description = "Background opacity (0.0-1.0).";
      };

      refresh_rate_ms = mkOption {
        type = types.int;
        default = 1000;
        description = "Refresh rate in milliseconds.";
      };
    };

    # -- Monitoring ------------------------------------------------------------
    monitoring = {
      show_cpu = mkOption {
        type = types.bool;
        default = true;
        description = "Show CPU usage panel.";
      };

      show_memory = mkOption {
        type = types.bool;
        default = true;
        description = "Show memory usage panel.";
      };

      show_disk = mkOption {
        type = types.bool;
        default = true;
        description = "Show disk usage panel.";
      };

      show_network = mkOption {
        type = types.bool;
        default = true;
        description = "Show network activity panel.";
      };

      show_gpu = mkOption {
        type = types.bool;
        default = false;
        description = "Show GPU usage panel (macOS only).";
      };
    };

    # -- Processes -------------------------------------------------------------
    processes = {
      sort_by = mkOption {
        type = types.enum [ "cpu" "memory" "pid" "name" ];
        default = "cpu";
        description = "Sort column for the process list.";
      };

      show_threads = mkOption {
        type = types.bool;
        default = false;
        description = "Show per-process threads.";
      };

      auto_refresh = mkOption {
        type = types.bool;
        default = true;
        description = "Auto-refresh the process list.";
      };
    };

    # -- Alerts ----------------------------------------------------------------
    alerts = {
      cpu_threshold = mkOption {
        type = types.float;
        default = 90.0;
        description = "CPU usage threshold percentage for alerts (0-100).";
      };

      memory_threshold = mkOption {
        type = types.float;
        default = 85.0;
        description = "Memory usage threshold percentage for alerts (0-100).";
      };

      disk_threshold = mkOption {
        type = types.float;
        default = 90.0;
        description = "Disk usage threshold percentage for alerts (0-100).";
      };
    };

    # -- Daemon ----------------------------------------------------------------
    daemon = {
      enable = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Run myaku as a persistent daemon (launchd on macOS, systemd on Linux).
          The daemon collects metrics and exposes a metrics endpoint.
        '';
      };

      metrics_port = mkOption {
        type = types.port;
        default = 9100;
        description = "Port for the metrics endpoint.";
      };

      history_retention_hours = mkOption {
        type = types.int;
        default = 24;
        description = "Hours of metric history to retain.";
      };
    };

    # -- Escape hatch ----------------------------------------------------------
    extraSettings = mkOption {
      type = types.attrs;
      default = {};
      description = ''
        Additional raw settings merged on top of typed options.
        Use this for experimental or newly-added config keys not yet
        covered by typed options. Values are serialized directly to YAML.
      '';
      example = {
        experimental = {
          gpu_backend = "metal";
        };
      };
    };
  };

  config = mkIf cfg.enable (mkMerge [
    # Install the package
    {
      home.packages = [ cfg.package ];
    }

    # Create log directory
    {
      home.activation.myaku-log-dir = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        run mkdir -p "${logDir}"
      '';
    }

    # YAML configuration -- always generated from typed options
    {
      xdg.configFile."myaku/myaku.yaml".source = yamlConfig;
    }

    # Darwin: launchd agent (daemon mode)
    (mkIf (cfg.daemon.enable && isDarwin)
      (mkLaunchdService {
        name = "myaku";
        label = "io.pleme.myaku";
        command = "${cfg.package}/bin/myaku";
        args = [ "daemon" ];
        logDir = logDir;
        processType = "Background";
        keepAlive = true;
      })
    )

    # Linux: systemd user service (daemon mode)
    (mkIf (cfg.daemon.enable && !isDarwin)
      (mkSystemdService {
        name = "myaku";
        description = "Myaku — system monitor daemon";
        command = "${cfg.package}/bin/myaku";
        args = [ "daemon" ];
      })
    )
  ]);
}
