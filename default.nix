{}:
let pkgs = import <nixpkgs> { };
in
pkgs.stdenv.mkDerivation {
  name = "termutils";
  src = ./.;

  buildInputs = with pkgs; [ gcc glibc clang clang-tools bear ];

  buildPhase = ''
    gcc -shared -o libtermutils.so termutils.c -fPIC -lm
    gcc -c termutils.c -o termutils.o -lm
    ar rcs libtermutils.a termutils.o
  '';

  installPhase = ''
    mkdir -p $out/lib
    mkdir -p $out/include

    cp libtermutils.so $out/lib/
    cp libtermutils.a $out/lib/
    cp termutils.h $out/include/
  '';
}
