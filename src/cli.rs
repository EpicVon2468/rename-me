use std::{ffi::OsString, path::PathBuf};

use inkwell::targets::{RelocMode, TargetMachine, TargetTriple as TargetTuple};

#[derive(Debug)]
pub struct UserConfig {
	pub jobs: u32,
	pub reloc: RelocMode,
	pub target: TargetTuple,
	pub verbose: bool,
	pub rpath: String,
	pub link_libraries: Vec<String>,
	pub dump_passes: bool,
	pub optimise: u32,
	pub emit_kind: EmitKind,
	pub output_file: Option<PathBuf>,
	pub input_files: Vec<PathBuf>,
	pub trailing: Vec<String>,
	// Could be a path or a path with arguments.
	pub linker_command: OsString,
	pub strip_command: OsString,
	pub opt_command: OsString,
}

impl Default for UserConfig {
	fn default() -> Self {
		Self {
			jobs: 1,
			reloc: Default::default(),
			target: TargetMachine::get_default_triple(),
			verbose: false,
			rpath: String::with_capacity(16),
			link_libraries: Vec::with_capacity(4),
			dump_passes: false,
			optimise: 1,
			emit_kind: Default::default(),
			output_file: None,
			input_files: Vec::with_capacity(4),
			trailing: Vec::with_capacity(4),
			linker_command: OsString::new(),
			strip_command: OsString::new(),
			opt_command: OsString::new(),
		}
	}
}

#[derive_const(Default)]
#[derive(Debug)]
pub enum EmitKind {
	#[default]
	ElfLinked,
	ElfObject,
	Assembly,
	IntermediateRepresentation,
}
