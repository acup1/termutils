{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  name = "termutils-dev";

  # ВАЖНО: nativeBuildInputs — для инструментов сборки
  nativeBuildInputs = with pkgs; [
    python313
    python313Packages.pybind11
    python313Packages.pip

    # КРИТИЧНО: правильный gcc с путями к glibc
    stdenv.cc.cc
  ];

  # buildInputs — для библиотек (не для компилятора)
  buildInputs = with pkgs; [ glibc.dev ];

  shellHook = ''
    [ -d venv ] || python3 -m venv venv
    source venv/bin/activate

    pip install --upgrade pip
    pip show pybind11 > /dev/null 2>&1 || pip install pybind11
    [ -f requirements.txt ] && pip install -r requirements.txt || true

    export EDITOR="nvim"
    export SHELL="${pkgs.zsh}/bin/zsh"
    export LD_LIBRARY_PATH=.:$LD_LIBRARY_PATH

    # Отладка: покажем, какие пути g++ видит
    echo "=== g++ include paths ==="
    echo | g++ -v -E -x c++ - 2>&1 | grep 'include.*glibc' || true

    $SHELL
  '';
}
