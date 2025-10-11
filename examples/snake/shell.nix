{ pkgs ? import <nixpkgs> { }, termutils ? import ../../default.nix { }, }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    gcc
    glibc
    glibc.static
    clang
    clang-tools
    bear
    termutils
  ];

  shellHook = ''
    export EDITOR="nvim"
    export SHELL="zsh"
    export CPATH="${pkgs.glibc.dev}/include:${pkgs.glibc.static}/include:${termutils}/include"
    export CPLUS_INCLUDE_PATH="${pkgs.glibc.dev}/include:${pkgs.glibc.static}/include:${termutils}/include"    
    export LIBRARY_PATH="${pkgs.glibc.dev}/lib:${pkgs.glibc.static}/lib:${termutils}/lib"   
    $SHELL
  '';
}
