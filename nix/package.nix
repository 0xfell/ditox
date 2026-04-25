{ lib
, rustPlatform
, pkg-config
, openssl
, wayland
, libxkbcommon
, glib
, cairo
, pango
, gdk-pixbuf
, atk
, gtk3
, xdotool
, libappindicator-gtk3
, libayatana-appindicator
, libdbusmenu-gtk3
, vulkan-loader
, libGL
, fontconfig
, freetype
, chafa
, xorg
, makeWrapper
}:

rustPlatform.buildRustPackage rec {
  pname = "ditox";
  version = "0.3.0";

  src = lib.cleanSource ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [ pkg-config makeWrapper ];

  buildInputs = [
    openssl
    # Clipboard + Wayland
    wayland
    libxkbcommon
    # Tray-icon + GTK
    glib
    cairo
    pango
    gdk-pixbuf
    atk
    gtk3
    xdotool
    libappindicator-gtk3
    libayatana-appindicator
    libdbusmenu-gtk3
    # Iced/winit runtime
    vulkan-loader
    libGL
    fontconfig
    freetype
    chafa
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libxcb
  ];

  # iced and tray-icon dlopen shared libs at runtime. Ensure they're on the
  # search path for the installed binaries.
  runtimeLibs = lib.makeLibraryPath [
    vulkan-loader
    libGL
    wayland
    libxkbcommon
    fontconfig
    freetype
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
  ];

  postInstall = ''
    for bin in $out/bin/*; do
      wrapProgram "$bin" --prefix LD_LIBRARY_PATH : "${runtimeLibs}"
    done
  '';

  # Tests use XDG_DATA_HOME / real filesystems and some depend on a display.
  doCheck = false;

  meta = with lib; {
    description = "Clipboard manager (TUI + GUI) for Wayland";
    longDescription = ''
      Ditox is a cross-platform clipboard manager with a terminal UI, a
      graphical UI (iced + tray icon), and a full CLI. On Linux it speaks
      Wayland natively via wl-clipboard; on Windows it uses arboard plus a
      Ctrl+Shift+V global hotkey. Image entries are content-addressed and
      stored with atomic writes and a refcount-backed prune queue.
    '';
    homepage = "https://github.com/0xfell/ditox";
    changelog = "https://github.com/0xfell/ditox/releases/tag/v${version}";
    license = licenses.mit;
    maintainers = [
      {
        name = "0xfell";
        github = "0xfell";
        githubId = 0; # Fill in if/when submitted to nixpkgs
      }
    ];
    mainProgram = "ditox";
    platforms = platforms.linux;
  };
}
