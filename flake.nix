{
  description = "Dev environment for C";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;
      in
      {
        devShells.default = pkgs.mkShell {
          name = "c-dev-shell";

          shell = pkgs.zsh;

          buildInputs = with pkgs; [
            # gcc
            # glibc
            # glibc.dev
            # glibc.static
            libgcc
            lcov
            check
            clang
            clang-tools
            cppcheck
            valgrind

            rustc
            cargo
            rustup
          ];

          shellHook = ''
            export LD_LIBRARY_PATH=${
              builtins.concatStringsSep ":" [
                "${pkgs.libgcc}/lib"
                "${pkgs.check}/lib"
              ]
            }:$LD_LIBRARY_PATH

            export CPATH=${
              builtins.concatStringsSep ":" [
                "${pkgs.libgcc}/include"
                "${pkgs.check}/include"
              ]
            }:$CPATH

            export CPLUS_INCLUDE_PATH="$CPATH"

            exec zsh
          '';

        };
      });
}
