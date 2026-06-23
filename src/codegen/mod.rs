use std::mem::ManuallyDrop;

use anyhow::Result;

use inkwell::attributes::AttributeLoc;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicMetadataTypeEnum, FunctionType};
use inkwell::values::FunctionValue;

use crate::errors::{ErrorSource, ICE, Phase};
use crate::map_llvm_type_to_metadata;
use crate::parser::fn_attrs::{
	Attributes,
	is_cold,
	is_force_inline,
	is_hot,
	is_strictfp,
	is_try_inline,
};
use crate::parser::function::FunctionDeclaration;
use crate::parser::modifiers::{Modifiers, is_external, is_private};
use crate::types::{__Type, AsLLVMType as _, LLVMType};

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

// Yes I know a separate impl is redundant, I just want it to be _very_ clear what I'm doing.
impl<'long> CodeGen<'long> {
	/// Returns [`Self`]'s LLVM module, with its [invariant lifetime] shortened from `'long` to `'short`.
	///
	/// # Safety
	///
	/// Callers must ensure that `'short` does not live longer than `'long`.
	///
	/// [invariant lifetime]: https://doc.rust-lang.org/nomicon/subtyping.html#variance
	unsafe fn coerce_module<'short>(&'short self) -> &'short Module<'short>
	where
		// 'long >= 'short
		'long: 'short, {
		// SAFETY:
		unsafe {
			std::mem::transmute::<&'short Module<'long>, &'short Module<'short>>(&self.module)
		}
	}
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

	pub fn create_function(&self, parsed: &FunctionDeclaration) -> Result<()> {
		// SAFETY: The borrow lifetime here does not live longer than 'ctx.
		unsafe { self.create_function_impl(parsed) }
	}

	/// # Safety
	///
	/// Callers must ensure that `'funct` does not live longer than `'ctx`.
	unsafe fn create_function_impl<'funct>(
		&'funct self,
		parsed: &FunctionDeclaration,
	) -> Result<()>
	where
		// 'ctx >= 'funct
		'ctx: 'funct, {
		let linkage: Option<Linkage> = {
			let modifiers: Modifiers = parsed.modifiers();
			if is_external(modifiers) {
				Some(Linkage::External)
			} else if is_private(modifiers) {
				Some(Linkage::Private)
			} else {
				None
			}
		};
		let return_type: LLVMType<'funct> = parsed.return_type().as_llvm_type(&self.context);
		let parameter_types: Vec<BasicMetadataTypeEnum<'funct>> = parsed
			.parameters()
			.iter()
			.map(|entry: &(_, __Type)| {
				map_llvm_type_to_metadata!(entry.1.as_llvm_type(&self.context))
			})
			.collect();
		let ty: FunctionType<'funct> = return_type
			.fn_type(&parameter_types, false)
			// FIXME: https://github.com/TheDan64/inkwell/pull/697#issuecomment-4760915392
			.map_err(|_| {
				ICE::new(ErrorSource::new(
					"[unimplemented: name of input file]",
					Phase::CodeGen,
				))
			})?;

		// SAFETY: Callers ensure 'funct <= 'ctx.
		let module: &'funct Module<'funct> = unsafe { self.coerce_module() };
		let value: FunctionValue<'funct> = module.add_function(parsed.identifier(), ty, linkage);
		{
			let attributes: Attributes = parsed.attributes();
			macro_rules! add_function_attr {
				($attr:literal) => {
					value.add_attribute(
						AttributeLoc::Function,
						self.context.create_string_attribute($attr, ""),
					);
				};
			}
			if is_cold(attributes) {
				add_function_attr!("cold");
			};
			if is_hot(attributes) {
				add_function_attr!("hot");
			};
			if is_strictfp(attributes) {
				add_function_attr!("strictfp");
			};
			if is_try_inline(attributes) {
				add_function_attr!("inlinehint");
			};
			if is_force_inline(attributes) {
				add_function_attr!("alwaysinline");
			};
		};
		Ok(())
	}

	pub fn debug(&self) {
		self.module.print_to_stderr();
	}
}
