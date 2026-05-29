{
  description = "Myaku (脈) — GPU system monitor for macOS and Linux";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";
    crate2nix.url = "github:nix-community/crate2nix";
    flake-utils.url = "github:numtide/flake-utils";
    substrate = {
      url = "github:pleme-io/substrate";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    crate2nix,
    flake-utils,
    substrate,
  }:
    (import "${substrate}/lib/rust-tool-release-flake.nix" {
      inherit nixpkgs crate2nix flake-utils;
    }) {
      toolName = "myaku";
      src = self;
      repo = "pleme-io/myaku";

      # Migration to substrate module-trio + shikumiTypedGroups.
      # Standard kekkai-style template — typed groups for
      # appearance/monitoring/processes/alerts, withUserDaemon for the
      # `myaku daemon` metrics collector, withShikumiConfig for the
      # YAML config at ~/.config/myaku/myaku.yaml.
      module = {
        description = "Myaku (脈) — GPU system monitor";
        hmNamespace = "blackmatter.components";

        # Daemon: `myaku daemon` — metrics collector + endpoint.
        withUserDaemon = true;
        userDaemonSubcommand = "daemon";

        # Shikumi YAML config at ~/.config/myaku/myaku.yaml.
        withShikumiConfig = true;

        shikumiTypedGroups = {
          appearance = {
            width           = { type = "int";   default = 800;  description = "Window width in pixels."; };
            height          = { type = "int";   default = 600;  description = "Window height in pixels."; };
            font_size       = { type = "float"; default = 13.0; description = "Font size in points."; };
            opacity         = { type = "float"; default = 0.95; description = "Background opacity (0.0-1.0)."; };
            refresh_rate_ms = { type = "int";   default = 1000; description = "Refresh rate in milliseconds."; };
          };

          monitoring = {
            show_cpu     = { type = "bool"; default = true;  description = "Show CPU usage panel."; };
            show_memory  = { type = "bool"; default = true;  description = "Show memory usage panel."; };
            show_disk    = { type = "bool"; default = true;  description = "Show disk usage panel."; };
            show_network = { type = "bool"; default = true;  description = "Show network activity panel."; };
            show_gpu     = { type = "bool"; default = false; description = "Show GPU usage panel (macOS only)."; };
          };

          processes = {
            sort_by = {
              type = nixpkgs.lib.types.enum [ "cpu" "memory" "pid" "name" ];
              default = "cpu";
              description = "Sort column for the process list.";
            };
            show_threads = { type = "bool"; default = false; description = "Show per-process threads."; };
            auto_refresh = { type = "bool"; default = true;  description = "Auto-refresh the process list."; };
          };

          alerts = {
            cpu_threshold    = { type = "float"; default = 90.0; description = "CPU usage threshold percentage for alerts (0-100)."; };
            memory_threshold = { type = "float"; default = 85.0; description = "Memory usage threshold percentage for alerts (0-100)."; };
            disk_threshold   = { type = "float"; default = 90.0; description = "Disk usage threshold percentage for alerts (0-100)."; };
          };
        };

        # The legacy module exposed daemon.{metrics_port,history_retention_hours}
        # alongside daemon.enable. The trio's withUserDaemon owns
        # daemon.{enable,extraArgs,environment}, so we move the bespoke
        # daemon-only fields into a sibling group `daemon_settings` and
        # merge them into the YAML at the legacy `daemon` key.
        extraHmOptions = {
          extraSettings = nixpkgs.lib.mkOption {
            type = nixpkgs.lib.types.attrs;
            default = { };
            description = "Additional raw settings merged on top of the typed YAML.";
          };
          daemon_settings = {
            metrics_port = nixpkgs.lib.mkOption {
              type = nixpkgs.lib.types.port;
              default = 9100;
              description = "Port for the metrics endpoint (daemon).";
            };
            history_retention_hours = nixpkgs.lib.mkOption {
              type = nixpkgs.lib.types.int;
              default = 24;
              description = "Hours of metric history to retain (daemon).";
            };
          };
        };

        extraHmConfigFn = { cfg, lib, ... }:
          let
            daemonExtras =
              if cfg.daemon.enable
              then {
                daemon = {
                  enable = true;
                  metrics_port = cfg.daemon_settings.metrics_port;
                  history_retention_hours = cfg.daemon_settings.history_retention_hours;
                };
              }
              else { };
            extras = daemonExtras // cfg.extraSettings;
          in lib.mkIf (extras != { }) {
            services.myaku.settings = extras;
          };
      };
    };
}
