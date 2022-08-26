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
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, fenix, naersk, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # Cross-compilation targets
        buildTargets = import ./targets.nix {inherit pkgs;};
        cargoTargets = buildTargets.cargoTargets;
        systemToTarget = buildTargets.systemToTarget;

        toolchain = with fenix.packages.${system};
          combine ([
            minimal.rustc
            minimal.cargo
          ] ++ builtins.map (cargoTarget: targets.${cargoTarget}.latest.rust-std) (builtins.attrNames cargoTargets));

        naersk' = naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        };

        naerskBuildPackage = target: args:
          naersk'.buildPackage (
            args
            // { CARGO_BUILD_TARGET = target; }
          );
      in
      rec {
        defaultPackage = packages.${systemToTarget.${system}};
        packages = builtins.mapAttrs naerskBuildPackage cargoTargets;
      }
    );
}
