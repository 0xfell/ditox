{
  description = "Ditox clipboard (Rust) - Nix flake";

  inputs = {
    # Use a recent nixpkgs to get a modern Cargo (lockfile v4)
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        # Use crane's mkLib to support all crane versions
        craneLib = crane.mkLib pkgs;
        # Important: include the full working tree (including untracked changes)
        # so local development files like new modules are available to the build.
        # This avoids git-index filtering that can miss newly added files.
        src = builtins.path { path = ./.; name = "ditox-src"; };
        # Build only the CLI without its default (libsql) feature to avoid
        # compiling libsql-ffi in sandboxed CI environments.
        # The CLI already enables the needed ditox-core features explicitly.
        commonArgs = rec {
          inherit src;
          pname = "ditox";
          version = "1.0.2";
          cargoToml = ./Cargo.toml;
          cargoLock = ./Cargo.lock;
          # Build just the CLI package and disable its default features
          # so that libsql (and libsql-ffi) are not pulled in for the Nix build.
          cargoExtraArgs = "-p ditox-cli --no-default-features";
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.xorg.libX11 pkgs.wayland pkgs.libxkbcommon ];
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        ditox = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
        # Tray-enabled variant for Wayland/SNI: add GTK3 + AppIndicator and enable feature `tray`.
        commonTray = commonArgs // {
          cargoExtraArgs = "-p ditox-cli --no-default-features --features tray";
          buildInputs = commonArgs.buildInputs ++ [ pkgs.gtk3 pkgs.libayatana-appindicator ];
        };
        cargoArtifactsTray = craneLib.buildDepsOnly commonTray;
        ditox-tray = craneLib.buildPackage (commonTray // { cargoArtifacts = cargoArtifactsTray; });
      in {
        packages.default = ditox;
        packages.ditox = ditox;
        packages.ditox-tray = ditox-tray;
        apps.default = {
          type = "app";
          program = "${ditox}/bin/ditox-cli";
        };
        apps.tray = {
          type = "app";
          program = "${ditox-tray}/bin/ditox-cli";
        };
        devShells.default = pkgs.mkShell {
          inputsFrom = [ ditox ];
          nativeBuildInputs = [ pkgs.rustc pkgs.cargo pkgs.clippy pkgs.rustfmt pkgs.pkg-config ];
          buildInputs = [ pkgs.xorg.libX11 pkgs.wayland pkgs.libxkbcommon pkgs.gtk3 pkgs.libayatana-appindicator ];
        };
      }
    );
}
