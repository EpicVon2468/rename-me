use std::mem::ManuallyDrop;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;

#[derive(Debug)]
pub struct CodeGen<'a> {
	// SANITY: ManuallyDrop is 0-cost.  Even if this looks verbose, it isn't.
	// SAFETY:
	// Problem(s):
	// - Using `ManuallyDrop` values after they have been dropped is Undefined Behaviour.
	// Excuse(s):
	// - Calling `ManuallyDrop::drop()` on this value is strictly prohibited, and does not occur until `Self` is dropped.
	context: ManuallyDrop<Context>,
	module: Module<'a>,
	builder: Builder<'a>,
}

impl CodeGen<'_> {
	#[must_use]
	pub fn new(name: &str) -> Self {
		let context: ManuallyDrop<Context> = ManuallyDrop::new(Context::create());
		let ptr: *const ManuallyDrop<Context> = &raw const context;
		// SANITY + SAFETY: `context` lives for as long as `Self`.
		let module: Module = unsafe { (*ptr).create_module(name) };
		// SANITY + SAFETY: `context` lives for as long as `Self`.
		let builder: Builder = unsafe { (*ptr).create_builder() };
		Self {
			context,
			module,
			builder,
		}
	}
}
