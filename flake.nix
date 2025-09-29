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
        commonArgs = rec {
          inherit src;
          pname = "ditox";
          version = "0.1.0";
          cargoToml = ./Cargo.toml;
          cargoLock = ./Cargo.lock;
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
