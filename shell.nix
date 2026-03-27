{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    openssl
    openssl.dev
    pkg-config
    alsa-lib
  ];
}
