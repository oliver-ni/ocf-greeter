{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    crane.url = "github:ipetkov/crane";
    systems.url = "github:nix-systems/default";
  };

  outputs = { self, nixpkgs, crane, systems }:
    let
      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [ self.overlays.default ];
      };

      forAllSystems = fn: nixpkgs.lib.genAttrs
        (import systems)
        (system: fn (pkgsFor system));
    in
    {
      overlays.default = final: prev: {
        ocf-greeter = final.callPackage ./. {
          craneLib = crane.mkLib final;
        };
      };

      packages = forAllSystems (pkgs: {
        default = pkgs.ocf-greeter;
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.git
            pkgs.dioxus-cli
            pkgs.tailwindcss
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.AppKit
            pkgs.darwin.apple_sdk.frameworks.WebKit
          ];
        };
      });
    };
}
