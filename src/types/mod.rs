pub mod floats;
pub mod integers;

use std::fmt::Debug;

use inkwell::context::Context;
use inkwell::types::AnyTypeEnum;

pub type LLVMType<'ctx> = AnyTypeEnum<'ctx>;

#[macro_export]
macro_rules! map_llvm_type_to_metadata {
	($input:expr) => {{
		use inkwell::types::BasicMetadataTypeEnum;

		let result: BasicMetadataTypeEnum<'_> = $crate::map_llvm_type!($input; ArrayType, FloatType, IntType, PointerType, StructType, VectorType, ScalableVectorType);
		result
	}};
}

#[macro_export]
macro_rules! map_llvm_type {
	($input:expr; $($valid:ident),* $(,)?) => {
		match $input {
			$(LLVMType::$valid(inner) => inner.into(),)*
			other => panic!("ICE: Unexpected or invalid type '{other}'!"),
		}
	};
}

pub type __Type = Box<dyn AsLLVMType>;

pub const trait AsLLVMType: Debug {
	fn as_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx>;
}

impl AsLLVMType for &dyn AsLLVMType {
	fn as_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx> {
		(*self).as_llvm_type(context)
	}
}

impl AsLLVMType for __Type {
	fn as_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx> {
		(**self).as_llvm_type(context)
	}
}

crate::zst_singleton!(__Void, void_type);
crate::zst_singleton!(__Boolean, bool_type);
crate::zst_singleton!(__Ptr);

impl AsLLVMType for __Ptr {
	fn as_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx> {
		context.ptr_type(Default::default()).into()
	}
}

// https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts
#[macro_export]
macro_rules! zst_singleton {
	($__type:ident, $type_fn:ident $(,)? $(#[$attr:meta])*) => {
		$crate::zst_singleton!($__type $(#[$attr])*);
		$crate::simple_type_impl!($__type, $type_fn);
	};
	($__type:ident $(,)? $(#[$attr:meta])*) => {
		$(#[$attr])*
		#[derive(Debug)]
		#[must_use]
		pub struct $__type;

		#[automatically_derived]
		const impl PartialEq for $__type {
			#[inline(always)]
			fn eq(&self, _: &Self) -> bool {
				true
			}
			#[inline(always)]
			fn ne(&self, _: &Self) -> bool {
				false
			}
		}
		#[automatically_derived]
		const impl Eq for $__type {}

		#[automatically_derived]
		const impl Default for $__type {
			#[doc = concat!("Delegates to [`", stringify!($__type), "::instance`].")]
			#[inline(always)]
			fn default() -> Self {
				Self::instance()
			}
		}

		impl $__type {
			#[doc = concat!("Returns the singleton instance of [`", stringify!($__type), "`].")]
			#[inline(always)]
			pub const fn instance() -> Self {
				Self
			}
		}

	};
}

#[macro_export]
macro_rules! simple_type_impl {
	($__type:ty, $type_fn:ident $(,)?) => {
		#[automatically_derived]
		impl $crate::types::AsLLVMType for $__type {
			#[inline(always)]
			fn as_llvm_type<'ctx>(
				&self,
				context: &'ctx inkwell::context::Context,
			) -> $crate::types::LLVMType<'ctx> {
				context.$type_fn().into()
			}
		}
	};
}
