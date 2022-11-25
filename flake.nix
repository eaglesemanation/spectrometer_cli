{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, fenix, naersk, utils, nixpkgs }:
    utils.lib.eachSystem
      (builtins.attrValues {
        # For now only support linux as build system
        inherit (utils.lib.system) x86_64-linux aarch64-linux;
      })
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import ./lib.nix) ];
          };
          inherit (pkgs.lib) toUpper removeSuffix stringAsChars recursiveMerge;

          # Use C compiler that links statically (should be gcc with musl)
          inherit (pkgs.pkgsStatic.stdenv) cc;
          target = removeSuffix "-" cc.targetPrefix;
          env_target = toUpper (stringAsChars (x: if x == "-" then "_" else x) target);

          toolchain = with fenix.packages.${system};
            combine ([
              minimal.rustc
              minimal.cargo
              targets.${target}.latest.rust-std
            ]);

          naersk' = naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          };

          naerskBuildPackage = package: args:
            naersk'.buildPackage (
              recursiveMerge [
                {
                  root = ./.;
                  doCheck = true;
                  strictDeps = true;

                  nativeBuildInputs = [ cc ];

                  cargoBuildOptions = x: x ++ [ "-p" package ];
                  cargoTestOptions = x: x ++ [ "-p" package ];

                  CARGO_BUILD_TARGET = target;
                  CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
                }
                args
              ]
            );
        in
        rec {
          packages = {
            spectrometer_cli = naerskBuildPackage "spectrometer_cli" { };
          };

          defaultPackage = packages.spectrometer_cli;
        }
      );
}
