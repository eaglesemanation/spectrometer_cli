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

  outputs = { self, fenix, naersk, flake-utils, nixpkgs }: {
    flake-utils.lib.eachDefaultSystem (system: {
      defaultPackage =
        let
          pkgs = nixpkgs.legacePackages.${system};
          target = "x86_64-unknown-linux-musl";
          toolchain = with fenix.packages.${system};
            combine [
              minimal.rustc
              minimal.cargo
              targets.${target}.latest.rust-std
            ];
        in
        (naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        }).buildPackage {
          src = ./.;
          CARGO_BUILD_TARGET = target;
        };
    });
  };
}
