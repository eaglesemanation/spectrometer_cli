{ targetPlatform
, craneLib
, package
}:
craneLib.buildPackage {
  name = package;
  src = craneLib.cleanCargoSource ../..;
  cargoExtraFlags = "-p ${package}";

  CARGO_BUILD_TARGET = targetPlatform.config;
  CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
}
