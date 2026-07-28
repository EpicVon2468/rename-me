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
	const_ops,
	exitcode_exit_method
)]
#![doc = include_str!("../README.md")]
pub mod cli;
pub mod codegen;
pub mod errors;
mod ffi;
pub mod lexer;
pub mod macros;
pub mod parser;
#[cfg(test)]
pub mod tests;
pub mod types;

use std::{
	env::{Args, args, var_os as get_var},
	error::Error,
	iter::Take,
	process::ExitCode,
	str::Chars,
};

use anyhow::{Context as _, Result, bail};

use inkwell::{
	support::{enable_llvm_pretty_stack_trace, get_llvm_version, shutdown_llvm},
	targets::{
		InitializationConfig as InitialisationConfig,
		RelocMode,
		Target,
		TargetMachine,
		TargetMachineOptions,
		TargetTriple as TargetTuple,
	},
};

use crate::{
	cli::{EmitKind, UserConfig},
	codegen::CodeGen,
	errors::CLIError,
	lexer::Source,
	parser::{Parser, TopLevel, function::FunctionDeclaration},
};

pub fn main() {
	let exit_code: ExitCode = match main_impl() {
		Ok(exit_code) => exit_code,
		Err(error) => {
			eprintln!("{error}");
			eprintln!("Caused by:");
			error
				.chain()
				.skip(1)
				.for_each(|cause: &(dyn Error + 'static)| eprintln!("\t{}", cause));
			ExitCode::FAILURE
		},
	};
	exit_code.exit_process();
}

fn main_impl() -> Result<ExitCode> {
	let mut config: UserConfig = UserConfig::default();
	fn next(args: &mut Args, cached: &mut Vec<String>) -> Option<String> {
		cached.pop().or_else(|| args.next())
	}
	let mut args: Args = args();
	let mut cached: Vec<String> = Vec::with_capacity(4);
	// skip executable
	let _ = args.next();
	while let Some(arg) = next(&mut args, &mut cached) {
		let mut chars: Chars = arg.chars();
		let mut prefix: Take<&mut Chars> = chars.by_ref().take(2);
		macro_rules! short_num_arg {
			($field:ident, $default:expr $(,)?) => {{
				let mut value: String = chars.as_str().to_owned();
				if value.is_empty() {
					if let Some(next) = next(&mut args, &mut cached) {
						// next was not a value, therefore we can assume the value was omitted
						if next.chars().next() == Some('-') {
							cached.push(next);
							value.push($default);
						} else {
							value = next;
						};
					} else {
						// omitting the value is equal to '$default'
						value.push($default);
					};
				};
				// TODO: better error handling here.
				config.$field = value.parse()?;
			}};
		}
		match (prefix.next().unwrap(), prefix.next().unwrap()) {
			('-', 'j') => {
				short_num_arg!(jobs, '2');
			},
			('-', 'v') => {
				config.verbose = true;
			},
			('-', 'O') => {
				short_num_arg!(optimise, '3');
			},
			('-', 'V') => {
				println!(env!("CARGO_PKG_VERSION"));
				return Ok(ExitCode::SUCCESS);
			},
			('-', '-') => {
				let arg: &str = &{
					let mut arg: String = chars.as_str().to_owned();
					// Handle space separated long arguments.
					// SANITY(unusual): Borrowing rules make doing this any other way difficult.
					if arg.split_once('=') == None {
						if let Some(next) = next(&mut args, &mut cached) {
							if next.chars().next() == Some('-') {
								// current arg is flag
								cached.push(next);
							} else {
								// pull in the value
								arg = format!("{arg}={next}");
							};
						};
					};
					arg
				};
				// parse long-form args (always has `=` sign)
				match arg.split_once('=') {
					Some(("reloc", value)) => match value {
						"default" => {
							config.reloc = RelocMode::Default;
						},
						"static" => {
							config.reloc = RelocMode::Static;
						},
						"pic" => {
							config.reloc = RelocMode::PIC;
						},
						"dynamic-no-pic" => {
							config.reloc = RelocMode::DynamicNoPic;
						},
						invalid => bail!(CLIError::InvalidValue {
							arg_name: "--reloc",
							value: invalid.to_owned(),
							expected: Some(
								"one of: 'default' OR 'static' OR 'pic' OR 'dynamic-no-pic'",
							),
						}),
					},
					Some(("target", value)) => {
						// Validation handled later.
						config.target = TargetTuple::create(value);
					},
					Some(("optimise", value)) => {
						config.optimise =
							value.parse().expect("Failed to parse optimisation level!");
					},
					Some(("emit", value)) => match value {
						"elf-linked" => {
							config.emit_kind = EmitKind::ElfLinked;
						},
						"elf-obj" => {
							config.emit_kind = EmitKind::ElfObject;
						},
						"asm" => {
							config.emit_kind = EmitKind::Assembly;
						},
						"llvm-ir" => {
							config.emit_kind = EmitKind::IntermediateRepresentation;
						},
						invalid => bail!(CLIError::InvalidValue {
							arg_name: "--emit",
							value: invalid.to_owned(),
							expected: Some(
								"one of: 'elf-linked' OR 'elf-obj' OR 'asm' OR 'llvm-ir'",
							),
						}),
					},
					None => {
						// no '=', therefore a flag
						match arg {
							"native-target" => {
								config.target = TargetMachine::get_default_triple();
							},
							"verbose" => {
								config.verbose = true;
							},
							"dump-passes" => {
								// Can't run here immediately & return, as LLVM targets need to be initialised first.
								config.dump_passes = true;
							},
							"optimise" => {
								// `--optimise` is equivalent to `--optimise=3`
								config.optimise = 3;
							},
							"no-optimise" => {
								config.optimise = 0;
							},
							"version" => {
								println!(concat!("renamec v", env!("CARGO_PKG_VERSION")));
								return Ok(ExitCode::SUCCESS);
							},
							invalid => bail!(CLIError::UnknownArgument {
								arg_name: String::from(invalid),
							}),
						}
					},
					invalid => bail!(CLIError::UnknownArgument {
						arg_name: unsafe { invalid.unwrap_unchecked() }.0.to_owned(),
					}),
				}
			},
			_ => bail!(CLIError::UnknownArgument { arg_name: arg }),
		}
	}
	if let Some(value) = get_var("LD") {
		config.linker_command = value;
	} else if let Some(value) = get_var("LINKER") {
		config.linker_command = value;
	};
	if let Some(value) = get_var("STRIP") {
		config.strip_command = value;
	};
	if let Some(value) = get_var("OPT") {
		config.opt_command = value;
	};
	dbg!(&config);
	if let Some(exit_code) = initialise_targets(&config)? {
		return Ok(exit_code);
	};
	Ok(ExitCode::SUCCESS)
}

fn initialise_targets(config: &UserConfig) -> Result<Option<ExitCode>> {
	let init_config: InitialisationConfig = InitialisationConfig::default();
	Target::initialize_x86(&init_config);
	Target::initialize_aarch64(&init_config);
	Target::initialize_power_pc(&init_config);
	Target::initialize_riscv(&init_config);
	Target::initialize_loongarch(&init_config);
	if let Err(error) = Target::from_triple(&config.target) {
		return Err(error).context(CLIError::InvalidValue {
			arg_name: "--target",
			value: config.target.as_str().to_string_lossy().into_owned(),
			expected: Some("a valid target tuple"),
		});
	};
	if config.dump_passes {
		// Prevent soundness violation if we're on a target not officially supported.
		// SANITY(unusual): The error type of this function is String, which cannot be used in `?` sugar.
		if let Err(error) = Target::initialize_native(&init_config) {
			bail!(error);
		};
		// SAFETY: LLVM's native target has been initialised.
		unsafe {
			ffi::print_passes();
		};
		return Ok(Some(ExitCode::SUCCESS));
	};
	let target_arch: Target =
		Target::from_triple(&config.target).context("Invalid target tuple!")?;
	dbg!(target_arch.get_name());
	// This is a rather wasteful conversion, as the &str is immediately turned back into &CStr by inkwell.
	// However, the `inner()` function of `TargetMachineOptions` appears to be private, meaning that
	// using the raw LLVM-C API isn't possible.
	// TODO: Is now a good time to start vendoring inkwell?
	let target_options: TargetMachineOptions =
		TargetMachineOptions::default().set_cpu(&target_arch.get_name().to_string_lossy());
	Ok(None)
}

pub fn _test() -> Result<()> {
	let config: InitialisationConfig = InitialisationConfig::default();
	Target::initialize_x86(&config);
	Target::initialize_aarch64(&config);
	Target::initialize_power_pc(&config);
	Target::initialize_riscv(&config);
	Target::initialize_loongarch(&config);
	let target: Target = Target::from_name("x86-64").unwrap();
	let target_tuple: TargetTuple = TargetTuple::create("x86_64-pc-linux-unknown");
	let options: TargetMachineOptions = TargetMachineOptions::default().set_cpu("x86-64");
	let _target_machine: TargetMachine = target
		.create_target_machine_from_options(&target_tuple, options)
		.unwrap();
	// SAFETY:
	unsafe {
		ffi::print_passes();
	};
	{
		{
			let mut input: &[u8] = b"1 + 42.0f * 2;";
			let mut parser: Parser = Source::into(&mut input);
			let _ = dbg!(parser.parse_expr()?);
		};
		let function: FunctionDeclaration = {
			let mut input: &[u8] = include_bytes!("../examples/main");
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
