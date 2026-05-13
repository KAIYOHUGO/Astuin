{
  description = "Astuin";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
      ];
      systems = [ "x86_64-linux" ];
      perSystem =
        {
          config,
          pkgs,
          ...
        }:
        {
          devShells.default =
            with pkgs;
            mkShell {
              buildInputs = [
              ];
              nativeBuildInputs = [
                rustc
                cargo
                rustfmt
              ];
            };
          packages.default = pkgs.callPackage (
            {
              lib,
              rustPlatform,
            }:
            rustPlatform.buildRustPackage {
              pname = "astuin";
              version = "1.0.0";
              src = ./.;
              cargoHash = "sha256-naXwfER7oYqB6Za2aQcZTsPHn7QLLTHY31jFnAbBtjs=";
            }
          ) { };
        };
      flake = {
      };
    };
}
