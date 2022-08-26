{ pkgs }:
{
  # Define target specific configuration for cargo invocation
  cargoTargets = {
    aarch64-unknown-linux-musl = 
    let
      cross-compile = with pkgs.pkgsStatic.stdenv; !(isLinux && isAarch64);
      cc = with pkgs; if cross-compile then pkgsCross.aarch64-multiplatform-musl.stdenv.cc else pkgsStatic.stdenv.cc;
    in {
      src = ./.;
      doCheck = true;
      strictDeps = true;

      depsBuildBuild = with pkgs; [ cc ];
      CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = "${cc}/bin/aarch64-unknown-linux-musl-ld";
    };

    x86_64-unknown-linux-musl = 
    let
      cross-compile = with pkgs.pkgsStatic.stdenv; !(isLinux && isx86_64);
      cc = with pkgs; if cross-compile then pkgsCross.x86_64-linux-musl.stdenv.cc else pkgsStatic.stdenv.cc;
    in {
      src = ./.;
      doCheck = true;
      strictDeps = true;

      depsBuildBuild = with pkgs; [ cc ];
      CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${cc}/bin/x86_64-unknown-linux-musl-ld";
    };

    i686-unknown-linux-musl = 
    let
      cross-compile = with pkgs.pkgsStatic.stdenv; !(isLinux && isi686);
      cc = with pkgs; if cross-compile then pkgsCross.i686-linux-musl.stdenv.cc else pkgsStatic.stdenv.cc;
    in {
      src = ./.;
      doCheck = true;
      strictDeps = true;

      depsBuildBuild = with pkgs; [ cc ];
      CARGO_TARGET_I686_UNKNOWN_LINUX_MUSL_LINKER = "${cc}/bin/i686-unknown-linux-musl-ld";
    };

    aarch64-apple-darwin = 
    let
      cross-compile = with pkgs.pkgsStatic.stdenv; !((if isDarwin then true else abort "Cross-compilation to MacOS is only available from MacOS") && isAarch64);
      cc = with pkgs; if cross-compile then pkgsCross.aarch64-darwin.stdenv.cc else pkgsStatic.stdenv.cc;
    in{
      src = ./.;
      doCheck = true;
      strictDeps = true;

      depsBuildBuild = with pkgs; [cc];
      CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER = "${cc}/bin/aarch64-apple-darwin-ld";
    };

    x86_64-apple-darwin = 
    let
      cross-compile = with pkgs.pkgsStatic.stdenv; !((if isDarwin then true else abort "Cross-compilation to MacOS is only available from MacOS") && isx86_64);
      cc = with pkgs; if cross-compile then pkgsCross.x86_64-darwin.stdenv.cc else pkgsStatic.stdenv.cc;
    in {
      src = ./.;
      doCheck = true;
      strictDeps = true;

      depsBuildBuild = with pkgs; [cc];
      CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER = "${cc}/bin/x86_64-apple-darwin-ld";
    };

    x86_64-pc-windows-gnu = {
      src = ./.;
      # FIXME: Enable testing with wine
      doCheck = false;
      strictDeps = true;

      depsBuildBuild = with pkgs; [
        pkgsCross.mingwW64.stdenv.cc
        pkgsCross.mingwW64.windows.pthreads
      ];

      nativeBuildInputs = with pkgs; [
        # We need Wine to run tests:
        wineWowPackages.minimal
      ];

      # Tells Cargo that it should use Wine to run tests.
      # (https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner)
      CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = pkgs.writeScript "wine-wrapper" ''
        export WINEPREFIX="$(mktemp -d)"
        exec wine64 $@
      '';
    };
  };
  # Map each nix system to preferred target
  systemToTarget = {
    aarch64-darwin = "aarch64-apple-darwin";
    aarch64-linux = "aarch64-unknown-linux-musl";
    x86_64-darwin = "x86_64-apple-darwin";
    x86_64-linux = "x86_64-unknown-linux-musl";
    i686-linux = "i686-unknown-linux-musl";
  };
}
