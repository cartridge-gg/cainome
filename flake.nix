{
  description = "Nix for development of zellij";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, fenix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        { system, pkgs, ... }:
        let
            rustToolchain = pkgs.fenix.fromToolchainFile {
                file = ./rust-toolchain.toml;
                sha256 = "sha256-lMLAupxng4Fd9F1oDw8gx+qA0RuF7ou7xhNU8wgs0PU=";
            };
          # Add nightly toolchain for WASM builds (needed for wasi_ext feature)
          # nightlyToolchain = pkgs.fenix.latest.withComponents [
          #   "cargo"
          #   "rustc"
          #   "rust-std"
          # ];
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ fenix.overlays.default ];
          };

          formatter = pkgs.nixpkgs-fmt;

          devShells = {
            default = pkgs.mkShell {
              packages = with pkgs; [
                nixd
                zlib
                curl
                protobuf
                rustToolchain
                # nightlyToolchain
                vscode-extensions.vadimcn.vscode-lldb
              ];

              shellHook = ''
                export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library";
                export PATH=$HOME/.cargo/bin:$PATH
              '';
            };
          };
        };
    };
}
