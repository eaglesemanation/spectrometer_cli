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
          ccd_lcamv06_x86_64 = pkgsX86_64LinuxStatic.callPackage ./nix/cargoPackage.nix { package = "ccd_lcamv06"; };
          spectrometer_cli_x86_64 = pkgsX86_64LinuxStatic.callPackage ./nix/cargoPackage.nix {
            cargoArtifacts = ccd_lcamv06_x86_64;
            package = "spectrometer_cli";
          };

          pkgsAarch64LinuxStatic = import nixpkgs (commonCfg // {
            crossSystem.config = "aarch64-unknown-linux-musl";
          });
          ccd_lcamv06_aarch64 = pkgsAarch64LinuxStatic.callPackage ./nix/cargoPackage.nix { package = "ccd_lcamv06"; };
          spectrometer_cli_aarch64 = pkgsAarch64LinuxStatic.callPackage ./nix/cargoPackage.nix {
            cargoArtifacts = ccd_lcamv06_aarch64;
            package = "spectrometer_cli";
          };

          pkgsMingwW64 = import nixpkgs (commonCfg // {
            crossSystem.config = "x86_64-w64-mingw32";
          });
          ccd_lcamv06_mingwW64 = pkgsMingwW64.callPackage ./nix/cargoPackage.nix { package = "ccd_lcamv06"; };
          spectrometer_cli_mingwW64 = pkgsMingwW64.callPackage ./nix/cargoPackage.nix {
            cargoArtifacts = ccd_lcamv06_mingwW64;
            package = "spectrometer_cli";
          };
        in
        rec {
          legacyPackages.pkgsCross = {
            x86_64-linux = {
              ccd_lcamv06 = ccd_lcamv06_x86_64;
              spectrometer_cli = spectrometer_cli_x86_64;
            };
            aarch64-linux = {
              ccd_lcamv06 = ccd_lcamv06_aarch64;
              spectrometer_cli = spectrometer_cli_aarch64;
            };
            mingwW64 = {
              ccd_lcamv06 = ccd_lcamv06_mingwW64;
              spectrometer_cli = spectrometer_cli_mingwW64;
            };
          };

          packages = {
            spectrometer_cli = legacyPackages.pkgsCross.${localSystem}.spectrometer_cli;
            default = packages.spectrometer_cli;
          };
        }
      );
}
