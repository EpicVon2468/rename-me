pub mod floats;
pub mod integers;

use std::fmt::Debug;

use inkwell::context::Context;
use inkwell::types::{AnyTypeEnum, BasicMetadataTypeEnum, FunctionType};

pub type LLVMType<'ctx> = AnyTypeEnum<'ctx>;

#[macro_export]
macro_rules! map_llvm_type {
	($($valid:ident),*; $input:expr $(,)?) => {
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

#[derive(Debug)]
pub struct __Void;

impl __Void {
	#[must_use]
	pub const fn instance() -> Self {
		Self
	}
}

crate::simple_type_impl!(__Void, void_type);

// https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts
#[derive(Debug)]
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
				context.$type_fn().into()
			}

			fn dbg_info(&self) -> String {
				String::from(stringify!($__type))
			}
		}
	};
}
