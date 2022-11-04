{ pkgs }:
{
  # Define target specific configuration for cargo invocation
  cargoTargets = {
    aarch64-unknown-linux-musl =
      let
        cross-compile = with pkgs.stdenv; !(isLinux && isAarch64);
        pkgsTarget = if cross-compile then pkgs.pkgsCross.aarch64-multiplatform-musl else pkgs;
        inherit (pkgsTarget.pkgsStatic.stdenv) cc;
      in
      {
        src = ./.;
        doCheck = true;
        strictDeps = true;

        depsBuildBuild = [ cc ];
        CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = "${cc}/bin/aarch64-unknown-linux-musl-ld";
      };

    x86_64-unknown-linux-musl =
      let
        cross-compile = with pkgs.stdenv; !(isLinux && isx86_64);
        pkgsTarget = if cross-compile then pkgs.pkgsCross.x86_64-linux-musl else pkgs;
        inherit (pkgsTarget.pkgsStatic.stdenv) cc;
      in
      {
        src = ./.;
        doCheck = true;
        strictDeps = true;

        depsBuildBuild = [ cc ];
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${cc}/bin/x86_64-unknown-linux-musl-ld";
      };

    i686-unknown-linux-musl =
      let
        cross-compile = with pkgs.stdenv; !(isLinux && isi686);
        pkgsTarget = if cross-compile then pkgs.pkgsCross.i686-linux-musl else pkgs;
        inherit (pkgsTarget.pkgsStatic.stdenv) cc;
      in
      {
        src = ./.;
        doCheck = true;
        strictDeps = true;

        depsBuildBuild = [ cc ];
        CARGO_TARGET_I686_UNKNOWN_LINUX_MUSL_LINKER = "${cc}/bin/i686-unknown-linux-musl-ld";
      };

    aarch64-apple-darwin =
      let
        cross-compile = with pkgs.stdenv; !((if isDarwin then true else abort "Cross-compilation to MacOS is only available from MacOS") && isAarch64);
        pkgsTarget = if cross-compile then pkgs.pkgsCross.aarch64-darwin else pkgs;
        cc = pkgsTarget.stdenv.cc;
      in
      {
        src = ./.;
        doCheck = true;
        strictDeps = true;

        depsBuildBuild = [ cc ];
        CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER = "${cc}/bin/aarch64-apple-darwin-ld";
      };

    x86_64-apple-darwin =
      let
        cross-compile = with pkgs.stdenv; !((if isDarwin then true else abort "Cross-compilation to MacOS is only available from MacOS") && isx86_64);
        pkgsTarget = if cross-compile then pkgs.pkgsCross.x86_64-darwin else pkgs;
        cc = pkgsTarget.stdenv.cc;
      in
      {
        src = ./.;
        doCheck = true;
        strictDeps = true;

        depsBuildBuild = [ cc ];
        CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER = "${cc}/bin/x86_64-apple-darwin-ld";
      };

    x86_64-pc-windows-gnu =
      let
        pkgsTarget = pkgs.pkgsCross.mingwW64;
        cc = pkgsTarget.stdenv.cc;
      in
      {
        src = ./.;
        # FIXME: Enable testing with wine
        doCheck = false;
        strictDeps = true;

        depsBuildBuild = with pkgsTarget; [ cc windows.pthreads ];

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

    i686-pc-windows-gnu =
      let
        pkgsTarget = pkgs.pkgsCross.mingw32;
        cc = pkgsTarget.buildPackages.wrapCC (pkgsTarget.buildPackages.gcc.cc.overrideAttrs (oldAttr: rec{
          configureFlags = oldAttr.configureFlags ++ [
            # Rust panic does not work with SJLJ
            "--disable-sjlj-exceptions --with-dwarf2"
          ];
        }));
      in
      {
        src = ./.;
        # FIXME: Enable testing with wine
        doCheck = false;
        strictDeps = true;

        depsBuildBuild = [ cc ] ++ (with pkgsTarget.windows; [ pthreads mcfgthreads ]);

        # Include mcfgthread dll in result for ease of access
        postInstall = ''
          ln -s ${pkgsTarget.windows.mcfgthreads}/bin/mcfgthread-12.dll $out/bin/mcfgthread-12.dll
        '';

        nativeBuildInputs = with pkgs; [
          # We need Wine to run tests:
          wineWowPackages.minimal
        ];

        # libgcc_eh implicitly links to libmcfgthreads
        CARGO_TARGET_I686_PC_WINDOWS_GNU_RUSTFLAGS = "-Clink-args=-lmcfgthread";

        # Tells Cargo that it should use Wine to run tests.
        # (https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner)
        CARGO_TARGET_I686_PC_WINDOWS_GNU_RUNNER = pkgs.writeScript "wine-wrapper" ''
          export WINEPREFIX="$(mktemp -d)"
          exec wine $@
        '';
      };
  };
  # Map each nix system to preferred target
  systemToTarget = {
    aarch64-linux = "aarch64-unknown-linux-musl";
    x86_64-linux = "x86_64-unknown-linux-musl";
    i686-linux = "i686-unknown-linux-musl";
    aarch64-darwin = "aarch64-apple-darwin";
    x86_64-darwin = "x86_64-apple-darwin";
  };
}
