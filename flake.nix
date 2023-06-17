{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-23.05";
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:icewind1991/naersk?rev=6d245a3bbb2ee31ec726bb57b9a8b206302e7110";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.inputs.flake-utils.follows = "utils";
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    naersk,
    rust-overlay,
  }:
    utils.lib.eachDefaultSystem (system: let
      overlays = [ (import rust-overlay) ];
      pkgs = (import nixpkgs) {
        inherit system overlays;
      };
      lib = pkgs.lib;
      naersk' = pkgs.callPackage naersk {};
      src = lib.sources.sourceByRegex (lib.cleanSource ./.) ["Cargo.*" "(src)(/.*)?"];
      nearskOpt = {
        pname = "evdev-shortcut";
        root = src;
      };
    in rec {
      packages = {
        check = naersk'.buildPackage (nearskOpt // {
          mode = "check";
        });
        clippy = naersk'.buildPackage (nearskOpt // {
          mode = "clippy";
        });
        test = naersk'.buildPackage (nearskOpt // {
          mode = "test";
        });
      };

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [rustc cargo bacon cargo-edit cargo-outdated clippy cargo-audit cargo-msrv cargo-fuzz];
      };
    });
}
