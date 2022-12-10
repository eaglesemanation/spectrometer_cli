{ stdenv
, targetPlatform
, writeScript
, windows
, wine64
, craneLib
, package
}:
craneLib.buildPackage {
  name = package;
  src = craneLib.cleanCargoSource ../..;
  cargoExtraFlags = "-p ${package}";

  buildInputs = [ windows.pthreads ];
  depsBuildBuild = [ wine64 ];

  CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
  CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "${stdenv.cc.targetPrefix}cc";
  CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = writeScript "wine-wrapper" ''
    export WINEPREFIX="$(mktemp -d)"
    exec wine64 $@
  '';

  HOST_CC = "${stdenv.cc.nativePrefix}cc";
}
