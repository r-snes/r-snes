{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
    ...
  }: let
      forAllSystems = nixpkgs.lib.genAttrs [
        "aarch64-linux"
        "x86_64-linux"
      ];
  in {
    packages = forAllSystems (system: {
      default = nixpkgs.legacyPackages.${system}.callPackage ./nix/package.nix {};
    });

    devShells = forAllSystems (system: {
      default = nixpkgs.legacyPackages.${system}.callPackage ./nix/shell.nix {};
    });
  };
}
