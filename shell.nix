{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  name = "c-shell";
  buildInputs = with pkgs; [
    gcc
    glibc
    glibc.dev
    clang
    clang-tools
    bear
    python313
  ];

  shellHook = ''
    python3 -m venv venv
    source venv/bin/activate
    pip install -r requirement.txt

    export EDITOR="nvim"
    export SHELL="zsh"
    # export CPATH="${pkgs.glibc.dev}/include:venv/lib/python3.13/site-packages/pybind11/include"
    # export CPLUS_INCLUDE_PATH="${pkgs.glibc.dev}/include:venv/lib/python3.13/site-packages/pybind11/include"
    export LD_LIBRARY_PATH=.:$LD_LIBRARY_PATH
    $SHELL
  '';
}
