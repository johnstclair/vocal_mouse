{
  description = "Rust Sound";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    oxalica.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, oxalica }:
    with flake-utils.lib;
    eachSystem allSystems (system:
      let
        pkgs = nixpkgs.legacyPackages.${system}.extend oxalica.overlays.default;
      in rec {

        packages = {
          rust-sound-program = let
            rustPlatform = pkgs.makeRustPlatform {
              cargo = pkgs.rust-bin.stable.latest.minimal;
              rustc = pkgs.rust-bin.stable.latest.minimal;
            };
          in rustPlatform.buildRustPackage rec {
            name = "rust-sound";
            src = self;
            nativeBuildInputs = with pkgs; [ pkg-config ];
            buildInputs = with pkgs; [
              alsa-lib.dev
              xdotool
              udev.dev 
              xorg.libX11
              xorg.libXrandr
              xorg.libXcursor
              xorg.libxcb
              xorg.libXi
              wayland
              libxkbcommon
              libxkbcommon.dev
              vulkan-loader
              vulkan-tools
              glfw
              xorg.xf86videoamdgpu  # notice this line might not match your needs or desires
            ];
            cargoLock = { lockFile = ./Cargo.lock; };
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          };
        };
        defaultPackage = packages.rust-sound-program;
        formatter = pkgs.nixfmt;
      });
}
