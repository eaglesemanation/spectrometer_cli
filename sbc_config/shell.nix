{ pkgs ? import <nixpkgs> {} }:

(pkgs.buildFHSUserEnv {
  name = "buildroot-env";

  targetPkgs = pkgs: (with pkgs; [
    # Buildroot mandatory deps
    which util-linux gnumake binutils diffutils gcc bash patch gzip 
    bzip2 perl cpio unzip rsync file bc findutils wget openssl
    # Buildroot optional deps
    python3 cvs git mercurial rsync subversion
    asciidoc w3m ncurses5 pkg-config
  ]);
  multiPkgs = null;
  extraOutputsToInstall = [ "dev" ];

  profile = ''
    export NIX_SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
    export SSL_CERT_FILE="$NIX_SSL_CERT_FILE"
    export SYSTEM_CERTIFICATE_PATH="$NIX_SSL_CERT_FILE"
    export LANG=C.UTF-8
  '';

  runScript = "bash";
}).env
