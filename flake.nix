{
  description = "fileshred — honest secure-delete";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        fileshred = pkgs.rustPlatform.buildRustPackage {
          pname = "fileshred";
          version = "0.1.0";
          src = ./.;
          cargoLock = { lockFile = ./Cargo.lock; };
          meta = with pkgs.lib; {
            description = "Honest secure-delete CLI";
            license = licenses.mit;
            mainProgram = "fileshred";
          };
        };
      in {
        packages.default = fileshred;
        apps.default = flake-utils.lib.mkApp { drv = fileshred; };
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ rustc cargo rustfmt clippy ];
        };
      });
}
