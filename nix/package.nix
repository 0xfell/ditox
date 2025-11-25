{ lib
, rustPlatform
, pkg-config
, openssl
, wayland
}:

rustPlatform.buildRustPackage rec {
  pname = "ditox";
  version = "0.1.12";

  src = lib.cleanSource ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl wayland ];

  # Skip tests for now
  doCheck = false;

  meta = with lib; {
    description = "Terminal clipboard manager for Wayland";
    homepage = "https://github.com/oxfell/ditox";
    license = licenses.mit;
    maintainers = [ ];
    mainProgram = "ditox";
    platforms = platforms.linux;
  };
}
