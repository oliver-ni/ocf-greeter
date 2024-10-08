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
      overlays.default = final: prev: {
        ocf-greeter = final.callPackage ./. { };
      };

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          LD_LIBRARY_PATH = "${nixpkgs.lib.makeLibraryPath (with pkgs; [
            wayland
            libxkbcommon
            libGL
          ])}";
        };
      });
    };
}
