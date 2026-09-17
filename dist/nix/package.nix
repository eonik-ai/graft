# SPDX-License-Identifier: Apache-2.0
# nixpkgs-shaped derivation. The in-repo flake is the supported Nix path
# until a nixpkgs PR lands. File SHA of the GitHub tag archive is in
# ../SOURCE; fetchFromGitHub wants the unpacked NAR hash instead.
{
  lib,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
}:

rustPlatform.buildRustPackage rec {
  pname = "graft";
  version = "0.2.2";

  # fetchFromGitHub hashes the unpacked NAR, not the file in ../SOURCE.
  # Fill after: nix-prefetch-github eonik-ai graft --rev v0.2.2
  # cargoHash from the first failed nix-build. The in-repo flake is the
  # supported Nix path until a nixpkgs PR lands.
  src = fetchFromGitHub {
    owner = "eonik-ai";
    repo = "graft";
    rev = "v${version}";
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  cargoHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  cargoBuildFlags = [ "--bin" "graft" ];
  nativeBuildInputs = [ pkg-config ];
  doCheck = false;

  meta = with lib; {
    description = "Change the hook. Keep the body.";
    longDescription = ''
      Local-first composition workspace. Incremental compiler for video.
      Runtime: ffmpeg and ffprobe on PATH. graft shells out; it does not
      link x264.
    '';
    homepage = "https://github.com/eonik-ai/graft";
    license = licenses.asl20;
    mainProgram = "graft";
  };
}
