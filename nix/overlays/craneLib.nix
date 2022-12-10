{ crane }: final: prev:
let
  nix2rustTarget = nixTarget:
    let
      mapping = {
        "x86_64-w64-mingw32" = "x86_64-pc-windows-gnu";
      };
    in
    if builtins.hasAttr nixTarget mapping
    then builtins.getAttr nixTarget mapping
    else nixTarget;

  rustToolchain = prev.pkgsBuildHost.rust-bin.stable.latest.default.override {
    targets = [ (nix2rustTarget prev.targetPlatform.config) ];
  };

  craneLib = (crane.mkLib prev).overrideToolchain rustToolchain;

  assetsFilter = path: _type:
    builtins.match ".*assets/.*" path != null;
  testsFilter = path: _type:
    builtins.match ".*tests/.*" path != null;
  cleanCargoSource = src: prev.lib.cleanSourceWith {
    inherit src;
    filter = path: type:
      (craneLib.filterCargoSources path type) || (assetsFilter path type) || (testsFilter path type);
  };
in
{
  craneLib = craneLib // { 
    # Include all files under assets/ and tests/
    inherit cleanCargoSource;
  };
}
