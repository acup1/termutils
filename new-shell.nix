{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  name = "termutils-dev";

  # ВАЖНО: nativeBuildInputs — для инструментов сборки (включая стденв-враппер)
  nativeBuildInputs = with pkgs; [
    python313
    python313Packages.pybind11
    python313Packages.pip
    python313Packages.setuptools # Для setup.py

    # КРИТИЧНО: полный stdenv.cc для враппера компилятора/линкера
    stdenv.cc
  ];

  # buildInputs — для библиотек (runtime + dev)
  buildInputs = with pkgs; [
    glibc.dev
    gcc.cc.lib # libgcc_s.so для линковки
    binutils # ld с правильными путями
  ];

  shellHook = ''
    # Создаём venv, если нет
    [ -d venv ] || python3 -m venv venv
    source venv/bin/activate

    # Установка Python-пакетов
    pip install --upgrade pip setuptools wheel
    pip install -r requirements.txt  # Если есть

    # КРИТИЧНО: Экспорт переменных для Nix-враппера (setup.py их использует)
    export CC="${pkgs.stdenv.cc}/bin/cc"
    export CXX="${pkgs.stdenv.cc}/bin/c++"
    export LD="${pkgs.binutils}/bin/ld"
    export AR="${pkgs.binutils}/bin/ar"
    export NM="${pkgs.binutils}/bin/nm"
    export STRIP="${pkgs.binutils}/bin/strip"

    # Флаги для компиляции/линковки (пути к glibc, crt и т.д.)
    export NIX_CFLAGS_COMPILE="-I${pkgs.glibc.dev}/include"
    export NIX_LDFLAGS="-L${pkgs.glibc.out}/lib -L${pkgs.gcc.cc.lib}/lib -rpath ${pkgs.glibc.out}/lib:${pkgs.gcc.cc.lib}/lib"

    # Для pybind11: используй language_level вместо cxx_std (в setup.py)
    export PYBIND11_NOPYTHON=1  # Опционально, для чистоты

    export EDITOR="nvim"
    export SHELL="${pkgs.zsh}/bin/zsh"
    export LD_LIBRARY_PATH=${pkgs.glibc.out}/lib:$LD_LIBRARY_PATH

    # Отладка: покажем пути линкера и glibc
    echo "=== Compiler paths ==="
    echo "CC: $CC"
    echo "CXX: $CXX"
    echo "LD: $LD"
    echo "NIX_LDFLAGS: $NIX_LDFLAGS"
    echo ""
    echo "=== glibc CRT path (should exist) ==="
    ls -la ${pkgs.glibc.out}/lib/crt/crti.o || echo "crti.o MISSING!"
    echo ""
    echo "=== g++ verbose linking test ==="
    echo 'int main(){}' | $CXX -x c++ - -v -Wl,--verbose 2>&1 | grep -E '(crti.o|SEARCH_DIR|glibc)' || true
    echo ""
    echo "Now run: python setup.py build_ext --inplace"
    echo "Clean first: rm -rf build/ termutils*.so"

    $SHELL
  '';
}
