// SAFETY: The function declarations given below are in line with the header files of `librename-me-cc`.
#[link(name = "rename-me-cc")]
unsafe extern "C" {

	/// Prints available LLVM optimisation passes.
	///
	/// # Library
	///
	/// Source(s):
	///
	/// - `src/rename-me.c++` (`librename-me-cc`).
	///
	/// Declaration:
	///
	/// ```
	/// #include "llvm/Support/raw_ostream.h"
	/// #include "llvm/Passes/PassBuilder.h"
	///
	/// extern "C" void print_passes() {
	/// 	llvm::PassBuilder builder;
	/// 	builder.printPassNames(llvm::outs());
	/// }
	/// ```
	///
	/// # Safety
	///
	/// This function is generally safe to call after `libLLVM` has been initialised.
	pub fn print_passes();
}
