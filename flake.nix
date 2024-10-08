{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    systems.url = "github:nix-systems/default";
  };

  outputs = { self, nixpkgs, systems }:
    let
      pkgsFor = system: import nixpkgs { inherit system; };
      forAllSystems = fn: nixpkgs.lib.genAttrs (import systems) (system: fn (pkgsFor system));
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell rec {
          packages = [ pkgs.wayland pkgs.libxkbcommon ];
          LD_LIBRARY_PATH = "${nixpkgs.lib.makeLibraryPath packages}";
        };
      });
    };
}
