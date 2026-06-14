use inkwell::context::Context;

use crate::simple_type_impl;
use crate::types::{__Type, LLVMType};

macro_rules! integer_type {
	($__int:ident, $num_bits:literal, Some($type_fn:ident) $(,)?) => {
		integer_type!($__int, $num_bits, None);
		simple_type_impl!($__int, $type_fn);
	};
	($__int:ident, $num_bits:literal, None $(,)?) => {
		#[doc = concat!("Enumeration representation of the ", $num_bits, "-bit integer type.")]
		#[doc = ""]
		#[doc = "The signedness of the type represented is determined by the variant of this enum."]
		#[doc = ""]
		#[doc = "It is recommended to use the exposed functions (i.e. [`Self::new`] & [`Self::is_unsigned`]) as opposed to using the internal enumeration variants, as this representation may be refactored in future."]
		#[doc = ""]
		#[doc = "# Examples"]
		#[doc = ""]
		#[doc = "```"]
		#[doc = concat!("let u", $num_bits, "_type: ", stringify!($__int), " = ", stringify!($__int), "::unsigned();")]
		#[doc = concat!("let i", $num_bits, "_type: ", stringify!($__int), " = ", stringify!($__int), "::signed();")]
		#[doc = concat!("assert!(u", $num_bits, "_type.is_unsigned());")]
		#[doc = concat!("assert!(!i", $num_bits, "_type.is_unsigned());")]
		#[doc = "```"]
		#[derive_const(PartialEq, Eq)]
		#[derive(Debug)]
		#[must_use]
		pub enum $__int {
			#[doc = concat!("Signed ", $num_bits, "-bit integer type.")]
			Signed,
			#[doc = concat!("Unsigned ", $num_bits, "-bit integer type.")]
			Unsigned,
		}

		impl $__int {
			#[doc = "Whether [`Self`] represents an unsigned integer type."]
			pub const fn is_unsigned(&self) -> bool {
				matches!(self, Self::Unsigned)
			}

			#[doc = concat!("Creates a new instance of [`", stringify!($__int), "`] configured to represent a ", $num_bits, "-bit integer type with signedness specified by `is_unsigned`.")]
			pub const fn new(is_unsigned: bool) -> Self {
				if is_unsigned {
					Self::Unsigned
				} else {
					Self::Signed
				}
			}

			#[doc = concat!("Creates a new instance of [`", stringify!($__int), "`] configured to represent a signed ", $num_bits, "-bit integer type.")]
			pub const fn signed() -> Self {
				Self::new(false)
			}

			#[doc = concat!("Creates a new instance of [`", stringify!($__int), "`] configured to represent an unsigned ", $num_bits, "-bit integer type.")]
			pub const fn unsigned() -> Self {
				Self::new(true)
			}
		}
	};
}

integer_type!(__8BitInteger, 8, Some(i8_type));
integer_type!(__16BitInteger, 16, Some(i16_type));
integer_type!(__32BitInteger, 32, Some(i32_type));
integer_type!(__64BitInteger, 64, Some(i64_type));
integer_type!(__128BitInteger, 128, None);

impl __Type for __128BitInteger {
	// FIXME: `Context` doesn't (currently) use `LLVMInt128TypeInContext()` under the hood.  Instead they use a custom-width integer type.  Awaiting fix.
	fn provide_llvm_type<'ctx>(&self, context: &'ctx Context) -> LLVMType<'ctx> {
		use inkwell::llvm_sys::core::LLVMInt128TypeInContext;
		use inkwell::llvm_sys::prelude::LLVMTypeRef;
		use inkwell::types::IntType;

		// SAFETY: False positive.
		let raw_ty: LLVMTypeRef = unsafe { LLVMInt128TypeInContext(context.raw()) };
		// SAFETY:
		// Problem(s):
		// - It is Undefined Behaviour to pass an `LLVMTypeRef` parameter which does not represent an integer type.
		// Excuse(s):
		// - The passed parameter is a known valid `LLVMTypeRef` integer type.
		let result: IntType = unsafe { IntType::new(raw_ty) };
		result.into()
	}

	fn dbg_info(&self) -> String {
		String::from("__128BitInteger")
	}
}
