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
          terminalibGitHash = "sha256-IZYzxP+LvIGCq08Tg8mI5OcWiOybI7EuBMG9ryl1Q/s=";
          buildExample = { pname, source }:
            pkgs.rustPlatform.buildRustPackage {
              inherit pname;
              version = "0.1.0";
              src = source;
              cargoLock = {
                lockFile = source + "/Cargo.lock";
                outputHashes."termutils-0.1.0" = terminalibGitHash;
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
        default = self.apps.${system}.paint;
      });
    };
}
