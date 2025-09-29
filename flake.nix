{
  description = "Ditox clipboard (Rust) - Nix flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = crane.lib.${system};
        src = craneLib.cleanCargoSource ./.;
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

