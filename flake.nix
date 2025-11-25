{
  description = "Ditox - Terminal clipboard manager for Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in {
        packages.default = pkgs.callPackage ./nix/package.nix { };
        packages.ditox = self.packages.${system}.default;

        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            # For wl-clipboard-rs
            wayland
          ];

          RUST_BACKTRACE = 1;
          RUST_LOG = "ditox=debug";
        };
      }
    ) // {
      homeManagerModules.default = import ./nix/module.nix;
      homeManagerModules.ditox = self.homeManagerModules.default;

      overlays.default = final: prev: {
        ditox = self.packages.${prev.system}.default;
      };
    };
}
