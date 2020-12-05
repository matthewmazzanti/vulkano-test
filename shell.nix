{ pkgs ? import <nixpkgs> {}, ... }:
with pkgs;
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
    cmake
    python3
  ];

  APPEND_LIBRARY_PATH = stdenv.lib.makeLibraryPath [
    vulkan-loader
    xlibs.libX11
    xlibs.libXcursor
    xlibs.libXrandr
    xlibs.libXi
  ];

  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$APPEND_LIBRARY_PATH"
  '';
}
