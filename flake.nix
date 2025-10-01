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
        # Keep test fixtures like `crates/ditox-cli/tests/fixtures/*.b64`
        # by avoiding overly-aggressive source filtering.
        src = craneLib.path ./.;
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
      in {
        packages.default = ditox;
        packages.ditox = ditox;
        apps.default = {
          type = "app";
          program = "${ditox}/bin/ditox-cli";
        };
        devShells.default = pkgs.mkShell {
          inputsFrom = [ ditox ];
          nativeBuildInputs = [ pkgs.rustc pkgs.cargo pkgs.clippy pkgs.rustfmt pkgs.pkg-config ];
          buildInputs = [ pkgs.xorg.libX11 pkgs.wayland pkgs.libxkbcommon ];
        };
      }
    );
}
