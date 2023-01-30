{ callPackage }:
rec {
  ccd_lcamv06 = callPackage ./cargoPackage.nix { package = "ccd_lcamv06"; };
  spectrometer_cli = callPackage ./cargoPackage.nix {
    cargoArtifacts = ccd_lcamv06;
    package = "spectrometer_cli";
  };
}
