{
  description = "Runnable terminalib example applications";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          terminalibGitHash = "sha256-ujQXKF8HC1hiBEXHJ9Sg6fx7tqFSTQHdWIV5MYDvRt0=";
          buildExample = { pname, source }:
            pkgs.rustPlatform.buildRustPackage {
              inherit pname;
              version = "0.1.0";
              src = source;
              cargoLock = {
                lockFile = source + "/Cargo.lock";
                outputHashes."terminalib-0.1.0" = terminalibGitHash;
              };
            };
        in
        rec {
          snake = buildExample {
            pname = "terminalib-snake-example";
            source = ./snake;
          };
          paint = buildExample {
            pname = "terminalib-paint-example";
            source = ./paint;
          };
          life = buildExample {
            pname = "terminalib-life-example";
            source = ./life;
          };
          tgp = buildExample {
            pname = "terminalib-tgp-example";
            source = ./tgp;
          };
          default = paint;
        });

      apps = forAllSystems (system: {
        snake = {
          type = "app";
          program = "${self.packages.${system}.snake}/bin/terminalib-snake-example";
        };
        paint = {
          type = "app";
          program = "${self.packages.${system}.paint}/bin/terminalib-paint-example";
        };
        life = {
          type = "app";
          program = "${self.packages.${system}.life}/bin/terminalib-life-example";
        };
        tgp = {
          type = "app";
          program = "${self.packages.${system}.tgp}/bin/terminalib-tgp-example";
        };
        default = self.apps.${system}.paint;
      });
    };
}
