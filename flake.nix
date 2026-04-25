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
        # Same derivation (it builds both binaries), aliased so users can
        # `nix profile install github:0xfell/ditox#ditox-gui` and have the
        # intent be explicit in their flake history.
        packages.ditox-gui = self.packages.${system}.default;

        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };
        apps.ditox = self.apps.${system}.default;
        apps.ditox-gui = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
          name = "ditox-gui";
        };

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
            openssl
            # Clipboard
            wl-clipboard
            wayland
            libxkbcommon
            # Tray (StatusNotifierItem via libappindicator/GTK)
            glib
            gdk-pixbuf
            cairo
            pango
            atk
            gtk3
            libappindicator-gtk3
            libayatana-appindicator   # tray-icon prefers this at runtime
            libdbusmenu-gtk3
            xdotool        # libxdo for muda/tray-icon
            # Iced / winit runtime (wgpu backend + tiny-skia fallback)
            vulkan-loader
            libGL
            fontconfig
            freetype
            expat
            chafa          # ratatui-image 10+ terminal graphics
            # X11 fallback (winit)
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libxcb
          ];

          # Iced/winit/tray-icon dlopen these at runtime; rpath them in
          LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
            vulkan-loader
            libGL
            wayland
            libxkbcommon
            fontconfig
            freetype
            # tray-icon uses libappindicator via dlopen on Linux
            libappindicator-gtk3
            libayatana-appindicator
            libdbusmenu-gtk3
            gtk3
            gdk-pixbuf
            glib
            atk
            cairo
            pango
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libxcb
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
