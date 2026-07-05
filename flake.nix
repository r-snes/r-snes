{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";
    rust-nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;
        rust-pkgs = import rust-nixpkgs { inherit system; };
        libs = with pkgs; [
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
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with rust-pkgs; [
            rustc
            cargo
            cargo-tarpaulin
            cargo-expand
            rustfmt
          ]
          ++ libs
          ++ (with pkgs; [
            pkg-config
            openssl
          ]);

          LD_LIBRARY_PATH = "${lib.makeLibraryPath libs}";
        };
      }
    );
}
