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

          # Iced/winit dlopen these at runtime; rpath them in
          LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
            vulkan-loader
            libGL
            wayland
            libxkbcommon
            fontconfig
            freetype
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
