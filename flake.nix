{
	description = "The `rename-me` compiler.";

	inputs = {
		nixpkgs = {
			type = "github";
			owner = "NixOS";
			repo = "nixpkgs";
			ref = "nixos-unstable";
		};
		flake-utils = {
			type = "github";
			owner = "numtide";
			repo = "flake-utils";
		};
	};

	outputs = { self, nixpkgs, flake-utils }:
		flake-utils.lib.eachDefaultSystem(system:
			let
				pkgs = import nixpkgs { inherit system; };
				llvmPkgs = pkgs.llvmPackages_22;
			in {
				system.isStatic = false;
				devShells.default = pkgs.mkShellNoCC {
					buildInputs = [
						llvmPkgs.llvm
						llvmPkgs.libllvm
					];
					LIB_LLVM = llvmPkgs.llvm.lib.outPath;
					DEV_LLVM = llvmPkgs.llvm.dev.outPath;
					LD_LIBRARY_PATH = "${llvmPkgs.llvm.lib.outPath}/lib";
					LLVM_PREFIX = llvmPkgs.llvm.dev.outPath;
				};
			}
		);
}
