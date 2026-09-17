# SPDX-License-Identifier: Apache-2.0
{
  description = "graft: change the hook. Keep the body.";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in rec {
          graft = pkgs.rustPlatform.buildRustPackage {
            pname = "graft";
            version = "0.2.2";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [ "--bin" "graft" ];
            doCheck = false;
            nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/graft \
                --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.ffmpeg ]}
            '';
            meta = with pkgs.lib; {
              description = "Change the hook. Keep the body.";
              homepage = "https://github.com/eonik-ai/graft";
              license = licenses.asl20;
              mainProgram = "graft";
            };
          };
          default = graft;
        });
      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.graft}/bin/graft";
        };
      });
    };
}
