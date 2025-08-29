{}:
let
  pkgs = import <nixpkgs> { };
in
pkgs.stdenv.mkDerivation {
  name = "termutils";
  src = ./.;

  buildInputs = with pkgs; [
    gcc
    glibc
    clang
    clang-tools
    bear
  ];

  buildPhase = ''
    gcc -shared -o libtermutils.so termutils.c -fPIC -lm
  '';

  installPhase = ''
    mkdir -p $out/lib
    mkdir -p $out/include
    cp libtermutils.so $out/lib/
    cp termutils.h $out/include/
  '';
}
