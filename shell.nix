{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    gcc
    glibc
    clang
    clang-tools
    bear
  ];

  shellHook = ''
    export EDITOR="nvim"
    export SHELL="zsh"
    export CPATH="${pkgs.glibc.dev}/include"
    export CPLUS_INCLUDE_PATH="${pkgs.glibc.dev}/include"
    export LD_LIBRARY_PATH=.:$LD_LIBRARY_PATH
    $SHELL
  '';
}
