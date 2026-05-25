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
        lib = pkgs.lib;
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in {
        packages.default = pkgs.callPackage ./nix/package.nix { };
        packages.ditox = self.packages.${system}.default;

        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };
        apps.ditox = self.apps.${system}.default;

        # `nix fmt`
        formatter = pkgs.nixpkgs-fmt;

        # `nix flake check` smoke tests — keep lightweight (no `cargo test`
        # because the test suite writes to XDG_DATA_HOME and a couple of
        # tests need a display / clipboard).
        checks.build = self.packages.${system}.default;

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustToolchain
            pkg-config
          ];

          buildInputs = with pkgs; [
            # Clipboard
            wl-clipboard
            wayland
            libxkbcommon
          ];

          LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
            wayland
            libxkbcommon
          ]);

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
