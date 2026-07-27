#include "llvm/Support/raw_ostream.h"
#include "llvm/Passes/PassBuilder.h"

extern "C" void print_passes() {
	llvm::PassBuilder builder;
	builder.printPassNames(llvm::outs());
}