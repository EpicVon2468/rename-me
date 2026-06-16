pub mod floats;
pub mod integers;

use std::fmt::Debug;

use inkwell::context::Context;
use inkwell::types::{AnyTypeEnum, BasicMetadataTypeEnum, FunctionType};

pub type LLVMType<'ctx> = AnyTypeEnum<'ctx>;

#[macro_export]
macro_rules! map_llvm_type {
	($input:expr; $($valid:ident),* $(,)?) => {
		match $input {
			$(inkwell::types::AnyTypeEnum::$valid(inner) => inner.into(),)*
			other => panic!("Unexpected or invalid type '{other}'!"),
		}
	};
}

pub trait LLVMTypeExt<'ctx> {
	fn fn_type_ext(
		self,
		param_types: &[BasicMetadataTypeEnum<'ctx>],
		is_var_args: bool,
	) -> FunctionType<'ctx>;
}

impl<'ctx> LLVMTypeExt<'ctx> for LLVMType<'ctx> {
	fn fn_type_ext(
		self,
		param_types: &[BasicMetadataTypeEnum<'ctx>],
		is_var_args: bool,
	) -> FunctionType<'ctx> {
		match self {
			AnyTypeEnum::ArrayType(inner) => inner.fn_type(param_types, is_var_args),
			AnyTypeEnum::FloatType(inner) => inner.fn_type(param_types, is_var_args),
			AnyTypeEnum::IntType(inner) => inner.fn_type(param_types, is_var_args),
			AnyTypeEnum::PointerType(inner) => inner.fn_type(param_types, is_var_args),
			AnyTypeEnum::StructType(inner) => inner.fn_type(param_types, is_var_args),
			AnyTypeEnum::VectorType(inner) => inner.fn_type(param_types, is_var_args),
			AnyTypeEnum::ScalableVectorType(inner) => inner.fn_type(param_types, is_var_args),
			AnyTypeEnum::VoidType(inner) => inner.fn_type(param_types, is_var_args),
			#[allow(clippy::panic)]
			AnyTypeEnum::FunctionType(_) => panic!("Function type unexpectedly reached!"),
		}
	}
}

pub trait __Type: Debug {
	fn provide_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx>;

	fn dbg_info(&self) -> &str;
}

impl __Type for &dyn __Type {
	fn provide_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx> {
		(*self).provide_llvm_type(context)
	}

	fn dbg_info(&self) -> &str {
		(*self).dbg_info()
	}
}

impl __Type for Box<dyn __Type> {
	fn provide_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx> {
		(**self).provide_llvm_type(context)
	}

	fn dbg_info(&self) -> &str {
		(**self).dbg_info()
	}
}

crate::zst_singleton!(__Void, void_type);
crate::zst_singleton!(__Boolean, bool_type);

// https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts
#[macro_export]
macro_rules! zst_singleton {
	($__type:ident, $type_fn:ident $(,)?) => {
		#[derive(Debug)]
		#[must_use]
		pub struct $__type;

		impl $__type {
			pub const fn instance() -> Self {
				Self
			}
		}

		$crate::simple_type_impl!($__type, $type_fn);
	};
}

#[macro_export]
macro_rules! simple_type_impl {
	($__type:ty, $type_fn:ident $(,)?) => {
		impl $crate::types::__Type for $__type {
			#[inline(always)]
			fn provide_llvm_type<'ctx>(
				&self,
				context: &'ctx inkwell::context::Context,
			) -> $crate::types::LLVMType<'ctx> {
				context.$type_fn().into()
			}

			fn dbg_info(&self) -> &'static str {
				stringify!($__type)
			}
		}
	};
}
