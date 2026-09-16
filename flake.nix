{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      utils,
      naersk,
    }:
    (utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
      in
      {
        packages.default = naersk-lib.buildPackage {
          src = ./.;
          buildInputs = with pkgs; [
            pkg-config
            libinput
          ];
        };
        devShell =
          with pkgs;
          mkShell {
            buildInputs = [
              cargo
              rustc
              rustPackages.clippy
              pkg-config
              libinput
            ];
            nativeBuildInputs = [
              rustfmt
              just
            ];
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          };
      }
    ))
    // {
      nixosModules.default = import ./nixos { rota = self.packages.${nixpkgs.hostPlatform}.default; };
    };
}
