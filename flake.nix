{
  description = "Ditox fresh OpenTUI + Zig workspace";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      lib = nixpkgs.lib;
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = lib.genAttrs systems;
      packageFor = system:
        let
          pkgs = import nixpkgs { inherit system; };
          runtimePath = lib.makeBinPath [
            pkgs.bun
            pkgs.hyprland
            pkgs.wl-clipboard
          ];
          # Explicitly import the @opentui tree from the (normally gitignored)
          # local node_modules after `bun install --frozen-lockfile` in tui/.
          # This becomes a content-addressed input to the derivation, so the
          # final package always ships the exact native libs that match the
          # dist/index.js. The Zig CLI then launches with --no-install --cwd
          # to guarantee isolation from the user's global Bun cache.
          tuiOpenTuiModules = builtins.path {
            path = ./. + "/tui/node_modules/@opentui";
            name = "ditox-tui-opentui";
          };
        in
        pkgs.stdenv.mkDerivation {
          pname = "ditox";
          version = "0.0.0-local";
          src = ./.;

          nativeBuildInputs = [
            pkgs.makeBinaryWrapper
            pkgs.pkg-config
            pkgs.zig_0_16
            pkgs.bun
          ];
          buildInputs = [
            pkgs.sqlite
          ];

          dontConfigure = true;

          buildPhase = ''
            runHook preBuild

            export DITOX_SQLITE3_INCLUDE_DIR="${pkgs.sqlite.dev}/include"
            export DITOX_SQLITE3_LIB_DIR="${pkgs.sqlite.out}/lib"
            export ZIG_GLOBAL_CACHE_DIR="$TMPDIR/zig-cache"

            # The TUI is built outside Nix (bun run build in tui/) using the
            # versions pinned in tui/bun.lock. We vendor both the resulting
            # dist/ *and* the matching @opentui node_modules (containing the
            # exact native .so for 0.2.15). Combined with the --no-install
            # --cwd launch logic in the Zig CLI, this completely isolates the
            # TUI from the user's global Bun cache and prevents the
            # dumpStdoutBuffer ABI crash that broke Super+V.
            zig build -Doptimize=ReleaseSafe --prefix "$out"

            # Vendor the exact @opentui modules (with matching native .so for
            # the 0.2.15 that produced dist/index.js). The explicit
            # tuiOpenTuiModules input ensures they are present even though
            # node_modules is gitignored.
            mkdir -p $out/share/ditox/tui/node_modules/@opentui
            cp -r ${tuiOpenTuiModules}/* $out/share/ditox/tui/node_modules/@opentui/ || true

            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            runHook postInstall
          '';

          postFixup = ''
            wrapProgram "$out/bin/ditox" \
              --prefix PATH : "${runtimePath}" \
              --set-default DITOXD "$out/bin/ditoxd"
            wrapProgram "$out/bin/ditoxd" \
              --prefix PATH : "${runtimePath}"
          '';

          meta = {
            description = "Ditox clipboard manager fresh OpenTUI and Zig build";
            mainProgram = "ditox";
            platforms = systems;
          };
        };
    in
    {
      packages = forAllSystems (system: {
        default = packageFor system;
        ditox = self.packages.${system}.default;
      });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/ditox";
        };
      });

      homeManagerModules.default = { config, lib, pkgs, ... }:
        let
          cfg = config.programs.ditox;
        in
        {
          options.programs.ditox = {
            enable = lib.mkEnableOption "Ditox clipboard manager";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
              defaultText = lib.literalExpression "ditox.packages.${pkgs.stdenv.hostPlatform.system}.default";
              description = "Ditox package to install.";
            };

            systemd.enable = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Whether to run the Ditox clipboard watcher as a user service.";
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ cfg.package ];

            systemd.user.services.ditox = lib.mkIf cfg.systemd.enable {
              Unit = {
                Description = "Ditox clipboard watcher";
                After = [ "graphical-session.target" ];
                PartOf = [ "graphical-session.target" ];
              };

              Service = {
                # Proper persistent daemon (approved architectural plan + full implementation).
                # Single long-lived DB owner running the *full* clipboard capture loop
                # (captureImageFirst/captureText/add*/markWatcherSeen + guards) against its one conn.
                # This is the root-cause structural fix (no more multi-writer SQLITE_BUSY deaths for the watcher).
                ExecStart = "${cfg.package}/bin/ditoxd daemon";
                Restart = "on-failure";
                RestartSec = 2;
              };

              Install.WantedBy = [ "graphical-session.target" ];
            };
          };
        };

      checks = forAllSystems (system: {
        default = self.packages.${system}.default;
      });

      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
            packages = [
              pkgs.bun
              pkgs.zig_0_16
              pkgs.sqlite
              pkgs.pkg-config
              pkgs.wl-clipboard
              pkgs.hyprland
            ];

            SQLITE3_INCLUDE_DIR = "${pkgs.sqlite.dev}/include";
            SQLITE3_LIB_DIR = "${pkgs.sqlite.out}/lib";

            shellHook = ''
              export DITOX_SQLITE3_INCLUDE_DIR="$SQLITE3_INCLUDE_DIR"
              export DITOX_SQLITE3_LIB_DIR="$SQLITE3_LIB_DIR"
            '';
          };
        });
    };
}
