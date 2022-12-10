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
        # For now only support building from x64 Linux
        inherit (utils.lib.system) x86_64-linux;
      })
      (localSystem:
        let
          staticTarget = nixpkgs.legacyPackages.${localSystem}.pkgsStatic.targetPlatform.config;

          pkgsStatic = import nixpkgs {
            inherit localSystem;
            crossSystem.config = staticTarget;
            overlays = [
              (import rust-overlay)
              (import ./nix/overlays/craneLib.nix { inherit crane; })
            ];
          };
          # Creates an attrset of derivations from a list of Cargo packages
          buildStaticPkgs = pkgs: builtins.listToAttrs (builtins.map
            (package: {
              name = package;
              value = pkgsStatic.callPackage ./nix/cargoPkgBuilder/musl.nix { inherit package; };
            })
            pkgs);

          pkgsMingwW64 = import nixpkgs {
            inherit localSystem;
            crossSystem.config = "x86_64-w64-mingw32";
            overlays = [
              (import rust-overlay)
              (import ./nix/overlays/craneLib.nix { inherit crane; })
            ];
          };
          # Creates an attrset of derivations from a list of Cargo packages
          buildMingwW64Pkgs = pkgs: builtins.listToAttrs (builtins.map
            (package: {
              name = package;
              value = pkgsMingwW64.callPackage ./nix/cargoPkgBuilder/mingwW64.nix { inherit package; };
            })
            pkgs);
        in
        rec {
          legacyPackages.pkgsCross = {
            mingwW64 = buildMingwW64Pkgs [ "spectrometer_cli" ];
          };

          checks = {
            inherit (legacyPackages.pkgsCross.mingwW64) spectrometer_cli;
          };

          packages = buildStaticPkgs [ "spectrometer_cli" ] // {
            default = packages.spectrometer_cli;
          };
        }
      );
}
