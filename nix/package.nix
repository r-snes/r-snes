{
  lib,
  stdenv,
  rustPlatform,

  # build inputs
  wayland,
  libxkbcommon,
  libGL,
  libXcursor,
  libXrandr,
  libXi,
  libX11,
  SDL2,

  pkg-config,
}:

let
  cargo-toml = lib.importTOML ../Cargo.toml;
in
  rustPlatform.buildRustPackage (finalAttrs: {
    pname = "r-snes";
    version = cargo-toml.package.version;

    meta = {
      descriptioon = "Fast and accurate SNES emulator";
      homepage = "https://github.com/r-snes/r-snes";
      license = lib.licenses.mit;
      mainProgram = "r-snes";
      platforms = lib.platforms.linux;
    };

    src = lib.fileset.toSource {
      root = ../.;
      fileset = lib.fileset.unions [
        ../src

        ../cpu
        ../apu
        ../ppu
        ../bus
        ../plugins
        ../common
        ../product_order

        ../assets
        ../packaging/r-snes.desktop

        ../Cargo.toml
        ../Cargo.lock
      ];
    };

    cargoLock = {
      lockFile = "${finalAttrs.src}/Cargo.lock";
      outputHashes = {
        "gc-arena-0.5.3" = "sha256-zJbTHKVmwuhiLiE8FfIe9FcnXsbjzj4PVWiQWKPLLOY=";
        "piccolo-0.3.3" = "sha256-xV+v/cCbMb2hfahGG+3HBshFFgtT+5tPmSSugNmRuho=";
      };
    };

    nativeBuildInputs = [
      pkg-config
    ];

    buildInputs = [
      wayland
      libxkbcommon
      libGL
      libXcursor
      libXrandr
      libXi
      libX11
      SDL2
    ];

    postInstall = ''
      install -Dm644 packaging/r-snes.desktop "$out/share/applications/r-snes.desktop"
      install -Dm644 assets/r-snes.png "$out/share/icons/hicolor/512x512/apps/r-snes.png"
    '';
  })
