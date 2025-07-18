{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    rust-nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
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

          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          xorg.libX11
        ];
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with rust-pkgs; [
            rustc
            cargo
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
