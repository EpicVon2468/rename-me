use std::mem::ManuallyDrop;

use inkwell::attributes::AttributeLoc;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{AsTypeRef as _, BasicMetadataTypeEnum, FunctionType};
use inkwell::values::FunctionValue;

use crate::map_llvm_type;
use crate::parser::function::FunctionDeclaration;
use crate::parser::fn_attrs::{
	Attributes,
	is_cold,
	is_force_inline,
	is_hot,
	is_strictfp,
	is_try_inline,
};
use crate::parser::modifiers::{Modifiers, is_external, is_private};
use crate::types::{__Type, LLVMType, LLVMTypeExt as _};

#[derive(Debug)]
pub struct CodeGen<'ctx> {
	// SANITY(overhead + unusual + verbosity): ManuallyDrop is 0-cost.
	// SAFETY:
	// Problem(s):
	// - Using `ManuallyDrop` values after they have been dropped is Undefined Behaviour.
	// Excuse(s):
	// - Calling `ManuallyDrop::drop()` on this value is strictly prohibited, and does not occur until `Self` is dropped.
	context: ManuallyDrop<Context>,
	module: Module<'ctx>,
	builder: Builder<'ctx>,
}

impl<'ctx> CodeGen<'ctx> {
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

	/// # Safety
	///
	/// Lifetimes, amirite?
	pub unsafe fn create_function(&self, function: &FunctionDeclaration) {
		let linkage: Option<Linkage> = {
			let modifiers: Modifiers = function.modifiers();
			if is_external(modifiers) {
				Some(Linkage::External)
			} else if is_private(modifiers) {
				Some(Linkage::Private)
			} else {
				None
			}
		};
		let raw_type: LLVMType = function.return_type().provide_llvm_type(&self.context);
		let parameter_types: Vec<BasicMetadataTypeEnum> = function.parameters().iter().map(|entry: &(_, Box<dyn __Type>)| {
			let type_enum: LLVMType = entry.1.provide_llvm_type(&self.context);
			let mapped: BasicMetadataTypeEnum = map_llvm_type!(ArrayType, FloatType, IntType, PointerType, StructType, VectorType, ScalableVectorType; type_enum);
			mapped
		}).collect();
		let function_type: FunctionType = raw_type.fn_type_ext(&parameter_types, false);

		// SANITY(unsound + unusual + what-the-fuck) + SAFETY:
		// Problem(s):
		// - The value created here can outlive `self` and cause a use-after-free or other Undefined Behaviour.
		// Excuse(s):
		// - Neither references nor ownership of values created here shall be exposed outside `self` at any point for any reason.
		// - This ensures that when `self` is dropped, all dangling references and values are dropped as well.
		let value: FunctionValue<'ctx> = {
			use inkwell::llvm_sys::core::LLVMAddFunction;
			use inkwell::llvm_sys::prelude::LLVMValueRef;

			// SAFETY:
			// Problem(s):
			// - This is unsound and can cause Undefined Behaviour (see above).
			// Excuse(s):
			// - See above.
			let result: LLVMValueRef = unsafe {
				LLVMAddFunction(
					self.module.as_mut_ptr(),
					// SAFETY: `Parser` appends a null byte termination to the identifier.
					function.identifier().as_ptr().cast(),
					function_type.as_type_ref(),
				)
			};
			// SAFETY:
			// Problem(s):
			// - This is unsound and can cause Undefined Behaviour (see above).
			// Excuse(s):
			// - See above.
			unsafe { FunctionValue::new(result) }.unwrap()
		};
		if let Some(linkage) = linkage {
			value.set_linkage(linkage);
		};
		{
			let attrs: Attributes = function.attributes();
			macro_rules! add_function_attr {
				($attr:literal) => {
					value.add_attribute(
						AttributeLoc::Function,
						self.context.create_string_attribute($attr, ""),
					);
				};
			}
			if is_cold(attrs) {
				add_function_attr!("cold");
			};
			if is_hot(attrs) {
				add_function_attr!("hot");
			};
			if is_strictfp(attrs) {
				add_function_attr!("strictfp");
			};
			if is_try_inline(attrs) {
				add_function_attr!("inlinehint");
			};
			if is_force_inline(attrs) {
				add_function_attr!("alwaysinline");
			};
		};
		value.print_to_stderr();
	}

	pub fn debug(&self) {
		self.module.print_to_stderr();
	}
}
