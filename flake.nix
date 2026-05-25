{
  description = "Ditox fresh OpenTUI + Zig workspace";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
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

