// Group lints
#![warn(clippy::pedantic, clippy::nursery, clippy::suspicious)]
// Specific lints
#![warn(
	clippy::as_conversions,
	clippy::min_ident_chars,
	clippy::pattern_type_mismatch,
	clippy::use_self,
	clippy::unused_trait_names,
	clippy::create_dir,
	clippy::exit,
	clippy::float_cmp,
	clippy::float_cmp_const,
	clippy::while_float,
	clippy::integer_division,
	clippy::integer_division_remainder_used,
	clippy::unreadable_literal,
	clippy::unnecessary_literal_bound,
	clippy::missing_const_for_fn,
	clippy::needless_collect,
	clippy::needless_for_each,
	clippy::as_underscore,
	clippy::branches_sharing_code,
	clippy::infinite_loop,
	clippy::linkedlist,
	clippy::pub_use,
	clippy::wildcard_imports,
	clippy::uninlined_format_args,
	clippy::equatable_if_let,
	clippy::enum_glob_use,
	clippy::panic,
	clippy::panic_in_result_fn
)]
#![forbid(
	clippy::undocumented_unsafe_blocks,
	clippy::multiple_unsafe_ops_per_block,
	clippy::missing_safety_doc,
	unsafe_op_in_unsafe_fn,
	reason = "All unsafe code must be wrapped in one unsafe block per call, and be safety documented!"
)]
#![allow(clippy::tabs_in_doc_comments, reason = "Why???  Bad clippy!")]
#![allow(
	clippy::unnecessary_semicolon,
	reason = "Consistency & uniformity looks better!  Bad clippy!"
)]
#![allow(
	clippy::missing_errors_doc,
	clippy::missing_panics_doc,
	reason = "I'll get to writing doc comments when I get to them."
)]
#![allow(
	clippy::default_trait_access,
	clippy::upper_case_acronyms,
	reason = "Shush"
)]
#![allow(clippy::borrowed_box)]
#![feature(
	derive_const,
	const_cmp,
	const_trait_impl,
	debug_closure_helpers,
	const_option_ops,
	const_range,
	const_result_trait_fn,
	const_convert,
	const_default,
	const_ops
)]
#![doc = include_str!("../README.md")]
pub mod codegen;
pub mod errors;
pub mod lexer;
pub mod parser;
pub mod types;

use anyhow::Result;

use inkwell::support::{enable_llvm_pretty_stack_trace, get_llvm_version, shutdown_llvm};

use crate::codegen::CodeGen;
use crate::lexer::Source;
use crate::parser::function::FunctionDeclaration;
use crate::parser::{Parser, TopLevel};

#[macro_export]
macro_rules! unreachable_ice {
	($msg:expr, $src:ident $(,)?) => {{
		#[allow(unused_imports)]
		use anyhow::{Error as Anyhow, anyhow, bail};

		use $crate::errors::{ErrorSource, ICE};

		std::hint::cold_path();
		bail!(Anyhow::context(
			$crate::with_location!(anyhow!($msg)),
			ICE::new(ErrorSource::$src),
		));
	}};
}

#[macro_export]
macro_rules! const_num_env {
	($env:literal, $default:literal $(,)?) => {
		const {
			#[inline(always)]
			const fn mapper(value: &str) -> usize {
				<usize as std::str::FromStr>::from_str(value).unwrap_or($default)
			}
			let value: usize = option_env!($env).map_or($default, mapper);
			assert!(
				(0..=(isize::MAX.cast_unsigned())).contains(&value),
				concat!(
					"Numeric environment variable '",
					$env,
					"' must be valid for allocation!",
				),
			);
			value
		}
	};
}

#[macro_export]
macro_rules! with_location {
	($err:expr) => {{
		#[allow(unused_imports)]
		use anyhow::{Error as Anyhow, anyhow};

		Anyhow::context(
			anyhow!(concat!(
				"[",
				file!(),
				':',
				line!(),
				':',
				column!(),
				"] [compiler internal tracking; ignore this line]",
			)),
			$err,
		)
	}};
}

pub fn main() -> Result<()> {
	{
		{
			let mut input: &[u8] = b"1 + 42.0f * 2;";
			let mut parser: Parser = Source::into(&mut input);
			let _ = dbg!(parser.parse_expr()?);
		};
		let function: FunctionDeclaration = {
			// #(method[MyStruct])
			let mut input: &[u8] = b"#(hot, strictfp, force_inline) const funct main(): i32;";
			let mut parser: Parser = Source::into(&mut input);
			let TopLevel::Function(function): TopLevel = dbg!(parser.parse()?);
			function
		};
		{
			enable_llvm_pretty_stack_trace();
			let (major, minor, patch): (u32, u32, u32) = get_llvm_version();
			println!("Loading with LLVM version {major}.{minor}.{patch}...");
		};
		let codegen: CodeGen = CodeGen::new("test");
		codegen.create_function(&function)?;
		codegen.debug();
	};
	// SAFETY:
	// Problem(s):
	// - LLVM data after this a call to `shutdown_llvm()` is likely to segfault.
	// Excuse(s):
	// - All references to LLVM data are dropped before this point.
	unsafe {
		shutdown_llvm();
	};
	Ok(())
}
