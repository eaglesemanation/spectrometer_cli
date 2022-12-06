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

          # For statically linked binaries
          ccStatic = pkgs.pkgsStatic.stdenv.cc;
          targetStatic = removeSuffix "-" ccStatic.targetPrefix;

          rustToolchain = with fenix.packages.${system};
            combine ([
              minimal.rustc
              minimal.cargo
              targets.${targetStatic}.latest.rust-std
              targets.x86_64-pc-windows-gnu.latest.rust-std
            ]);

          cargoDerivationBuilder = naersk.lib.${system}.override {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          # Build binary dynamically linked to system libs
          cargoBuildPackage = package: args:
            cargoDerivationBuilder.buildPackage (
              recursiveMerge [
                {
                  root = ./.;
                  doCheck = true;
                  strictDeps = true;

                  cargoBuildOptions = x: x ++ [ "-p" package ];
                  cargoTestOptions = x: x ++ [ "-p" package ];
                }
                args
              ]
            );

          # Build binary statically linked to system libs
          cargoBuildPackageStatic = package: args:
            cargoBuildPackage package (recursiveMerge [
              {
                nativeBuildInputs = [ ccStatic ];
                CARGO_BUILD_TARGET = targetStatic;
              }
              args
            ]);

          # Cross compile to Windows
          cargoBuildWinPackage = package: args:
            cargoBuildPackage package (recursiveMerge [
              {
                depsBuildBuild = with pkgs.pkgsCross.mingwW64; [
                  stdenv.cc
                  windows.pthreads
                ];

                nativeBuildInputs = with pkgs;  [
                  winePackages.minimal
                ];

                doCheck = false;
                CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
                CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = pkgs.writeScript "wine-wrapper" ''
                  export WINEPREFIX="$(mktemp -d)"
                  exec wine64 $@
                '';
              }
            ]);
        in
        rec {
          legacyPackages.pkgsCross = {
            mingwW64 = {
              spectrometer_cli = cargoBuildWinPackage "spectrometer_cli" { };
            };
          };

          packages = {
            spectrometer_cli = cargoBuildPackageStatic "spectrometer_cli" { };
          };

          defaultPackage = packages.spectrometer_cli;

          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              pkgsCross.mingwW64.stdenv.cc
              pkgsCross.mingwW64.windows.pthreads
              winePackages.minimal
              rustToolchain
            ];
          };
        }
      );
}
