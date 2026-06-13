pub mod floats;
pub mod integers;

use inkwell::context::Context;
use inkwell::types::AnyType;

pub type LLVMType<'ctx> = Box<dyn AnyType<'ctx> + 'ctx>;

pub trait __Type {
	fn provide_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx>;

	fn dbg_info(&self) -> String;
}

impl __Type for &dyn __Type {
	fn provide_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx> {
		(*self).provide_llvm_type(context)
	}

	fn dbg_info(&self) -> String {
		(*self).dbg_info()
	}
}

impl __Type for Box<dyn __Type> {
	fn provide_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx> {
		(**self).provide_llvm_type(context)
	}

	fn dbg_info(&self) -> String {
		(**self).dbg_info()
	}
}

// https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts
pub struct __Boolean;

impl __Boolean {
	#[must_use]
	pub const fn instance() -> Self {
		Self
	}
}

crate::simple_type_impl!(__Boolean, bool_type);

#[macro_export]
macro_rules! simple_type_impl {
	($__type:ty, $type_fn:ident $(,)?) => {
		impl $crate::types::__Type for $__type {
			#[inline(always)]
			fn provide_llvm_type<'ctx>(
				&self,
				context: &'ctx inkwell::context::Context,
			) -> $crate::types::LLVMType<'ctx> {
				Box::new(context.$type_fn())
			}

			fn dbg_info(&self) -> String {
				String::from(stringify!($__type))
			}
		}
	};
}
