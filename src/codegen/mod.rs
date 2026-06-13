use std::mem::ManuallyDrop;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};

use crate::parser::FunctionDeclaration;
use crate::parser::attrs::{Attributes, is_external, is_private};

#[derive(Debug)]
pub struct CodeGen<'a> {
	// SANITY(overhead + unusual + verbosity): ManuallyDrop is 0-cost.
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
		// SANITY(ptr) + SAFETY: `context` lives for as long as `Self`.
		let module: Module = unsafe { (*ptr).create_module(name) };
		// SANITY(ptr) + SAFETY: `context` lives for as long as `Self`.
		let builder: Builder = unsafe { (*ptr).create_builder() };
		Self {
			context,
			module,
			builder,
		}
	}

	pub fn create_function(&mut self, function: &FunctionDeclaration) {
		let linkage: Option<Linkage> = {
			let attrs: Attributes = function.attributes;
			if is_external(attrs) {
				Some(Linkage::External)
			} else if is_private(attrs) {
				Some(Linkage::Private)
			} else {
				None
			}
		};

		self.module
			.add_function(&function.identifier, todo!(), linkage);
		// self.module.add_function();
	}

	pub fn debug(&self) {
		self.module.print_to_stderr();
	}
}
