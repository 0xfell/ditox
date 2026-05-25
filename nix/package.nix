{ lib
, rustPlatform
, pkg-config
, wayland
, libxkbcommon
}:

rustPlatform.buildRustPackage rec {
  pname = "ditox";
  version = "0.3.1";

  src = lib.cleanSource ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [ pkg-config ];

  buildInputs = [
    wayland
    libxkbcommon
  ];

  # Tests use XDG_DATA_HOME / real filesystems and some depend on a display.
  doCheck = false;

  meta = with lib; {
    description = "Terminal clipboard manager for Wayland";
    longDescription = ''
      Ditox is a terminal-first clipboard manager with a TUI, watcher daemon,
      and full CLI. It speaks Wayland natively via wl-clipboard. Image entries
      are content-addressed and stored with atomic writes and a refcount-backed
      prune queue.
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
