{
  description = "Liteflags-rs Core Library - Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            
            # Build dependencies
            pkg-config
            openssl
            
            # Development tools
            cargo-watch
            cargo-edit
            cargo-audit
            cargo-nextest  # Better test runner
            
            # System dependencies
            libiconv
            
            # Additional development tools
            jq           # JSON processing
            curl         # HTTP testing  
            git          # Version control
          ];

          # Environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          RUST_LOG = "debug";
          RUST_BACKTRACE = "1";
        };

        # Formatter for Nix files
        formatter = pkgs.nixpkgs-fmt;
      });
}
