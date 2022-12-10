{ lib
, stdenv
, targetPlatform
, windows
, craneLib
, package
, cargoArtifacts ? null
}:
let
  target = craneLib.nix2rustTarget targetPlatform.config;
  targetCaps = lib.toUpper (lib.stringAsChars (c: if c == "-" then "_" else c) target);
in
craneLib.buildPackage ({
  pname = package;
  src = craneLib.cleanCargoSource ./..;
  cargoExtraFlags = "-p ${package}";

  buildInputs = [
  ]
  ++ lib.optionals (targetPlatform.isWindows) [
    windows.pthreads
  ];

  CARGO_BUILD_TARGET = target;
  CARGO_BUILD_RUSTFLAGS = lib.optionalString (targetPlatform.isStatic) "-C target-feature=+crt-static";
  "CARGO_TARGET_${targetCaps}_LINKER" = "${stdenv.cc.targetPrefix}cc";

  HOST_CC = "${stdenv.cc.nativePrefix}cc";
} // lib.optionalAttrs (cargoArtifacts != null) {
  inherit cargoArtifacts;
})
