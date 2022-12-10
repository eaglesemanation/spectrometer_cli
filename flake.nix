{
  inputs = {
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, rust-overlay, crane, utils, nixpkgs }:
    utils.lib.eachSystem
      (builtins.attrValues {
        # For now only support building from Linux
        inherit (utils.lib.system) x86_64-linux aarch64-linux;
      })
      (localSystem:
        let
          commonCfg = {
            inherit localSystem;
            overlays = [
              (import rust-overlay)
              # Adds pkgs.craneLib with rust toolchain for cross compilation
              (import ./nix/craneLibOverlay.nix { inherit crane; })
            ];
          };

          pkgsX86_64LinuxStatic = import nixpkgs (commonCfg // {
            crossSystem.config = "x86_64-unknown-linux-musl";
          });

          pkgsAarch64LinuxStatic = import nixpkgs (commonCfg // {
            crossSystem.config = "aarch64-unknown-linux-musl";
          });

          pkgsMingwW64 = import nixpkgs (commonCfg // {
            crossSystem.config = "x86_64-w64-mingw32";
          });
        in
        rec {
          legacyPackages.pkgsCross = {
            mingwW64 = {
              spectrometer_cli = pkgsMingwW64.callPackage ./nix/cargoPackage.nix { package = "spectrometer_cli"; };
            };
            x86_64-linux = {
              spectrometer_cli = pkgsX86_64LinuxStatic.callPackage ./nix/cargoPackage.nix { package = "spectrometer_cli"; };
            };
            aarch64-linux = {
              spectrometer_cli = pkgsAarch64LinuxStatic.callPackage ./nix/cargoPackage.nix { package = "spectrometer_cli"; };
            };
          };

          packages = {
            spectrometer_cli = legacyPackages.pkgsCross.${localSystem}.spectrometer_cli;
            default = packages.spectrometer_cli;
          };
        }
      );
}
