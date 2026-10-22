{
  mkShell,

  # build inputs
  wayland,
  libxkbcommon,
  libGL,
  libXcursor,
  libXrandr,
  libXi,
  libX11,
  SDL2,

  # toolchain
  rustc,
  cargo,
  clippy,
  cargo-tarpaulin,
  cargo-expand,
  rustfmt,

  pkg-config,
}:

mkShell {
  name = "R-SNES dev shell";

  nativeBuildInputs = [
    rustc
    cargo
    clippy
    cargo-tarpaulin
    cargo-expand
    rustfmt

    pkg-config
  ];

  buildInputs = [
    wayland
    libxkbcommon
    libGL
    SDL2
    libXcursor
    libXrandr
    libXi
    libX11
    SDL2
  ];
}
