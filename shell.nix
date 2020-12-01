{ pkgs ? import <nixpkgs> {}, ... }:
with pkgs;
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
  ];
}
