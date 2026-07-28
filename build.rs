#![allow(non_snake_case)]
#![feature(exit_status_error)]

use std::{
	collections::VecDeque,
	env::var as env,
	fs::File,
	process::{Command, Output, Stdio},
};

use roff::{Inline, Roff, bold, italic, line_break, roman};

use shell_words::{ParseError, split as parse};

const PROGRAM: &'static str = "renamec";

fn compile_cxx() {
	let llvm_config: String = match env("LLVM_SYS_221_PREFIX") {
		Ok(prefix) => format!("{prefix}/bin/llvm-config"),
		Err(_) => String::from("llvm-config"),
	};
	let output: Output = Command::new(llvm_config)
		.arg("--cxxflags")
		.stdout(Stdio::piped())
		.spawn()
		.expect("Failed to run `llvm-config`!")
		.wait_with_output()
		.expect("Failed to get ouput of `llvm-config`!")
		.exit_ok()
		.expect("`llvm-config` returned non-zero!");
	let stdout: &str = str::from_utf8(&output.stdout)
		.expect("Failed to convert `llvm-config`'s stdout into UTF-8!");
	let Ok(flags): Result<Vec<String>, ParseError> = parse(stdout) else {
		unreachable!("Infeasible.");
	};
	cc::Build::new()
		.cpp(true)
		.warnings(false)
		.extra_warnings(false)
		.inherit_rustflags(false)
		.std("c++17")
		.file("src/rename-me.c++")
		.flags(flags)
		.compile("rename-me-cc");
}

pub fn main() {
	compile_cxx();
	let mut page: Roff = Roff::new();
	page.control(
		"TH",
		[
			"RENAMEC",
			"1",
			"2026-07-28",
			concat!("rename-me ", env!("CARGO_PKG_VERSION")),
		],
	);
	name_section(&mut page);
	synopsis_section(&mut page);
	description_section(&mut page);
	options_section(&mut page);
	environment_section(&mut page);
	see_also_section(&mut page);
	{
		let out_dir: String = {
			let target_dir: String = env("CARGO_TARGET_DIR")
				.unwrap_or_else(|_| format!("{}/target", env("CARGO_MANIFEST_DIR").unwrap()));
			format!("{target_dir}/{}", env("PROFILE").unwrap())
		};
		let file: String = format!("{out_dir}/{PROGRAM}.1");
		let mut out: File = File::options()
			.write(true)
			.truncate(true)
			.create(true)
			.open(&file)
			.unwrap();
		page.to_writer(&mut out).unwrap();
	};
}

fn man_ref(roff: &mut Roff, refs: &[(&str, &str)]) {
	debug_assert!(refs.len() >= 1);
	let &[ref rest @ .., last]: &[(&str, &str)] = refs else {
		let [single]: &[(&str, &str)] = refs else {
			unreachable!();
		};
		roff.control("BR", [single.0, &format!("({})", single.1)]);
		return;
	};
	for entry in rest {
		roff.control("BR", [entry.0, &format!("({}),", entry.1)]);
	}
	roff.control("BR", [last.0, &format!("({})", last.1)]);
}

fn dot_point(roff: &mut Roff) {
	roff.control("IP", ["\\(bu", "2"]);
}

fn name_section(roff: &mut Roff) {
	roff.control("SH", ["NAME"]);
	roff.control("PP", []);
	roff.text([roman(format!("{PROGRAM} – the rename-me compiler"))]);
}

// jobs
fn __jobs__short() -> [Inline; 3] {
	[bold("-j"), roman(" "), italic("n")]
}
fn __jobs__long() -> [Inline; 3] {
	[bold("--jobs"), roman("="), italic("n")]
}

// relocation mode
fn __reloc_mode__long() -> [Inline; 2] {
	[
		bold("--reloc"),
		roman("={default|static|pic|dynamic-no-pic}"),
	]
}

// target tuple
fn __target__long() -> [Inline; 3] {
	[bold("--target"), roman("="), italic("tuple")]
}
fn __target_native__long() -> [Inline; 1] {
	[bold("--native-target")]
}

// verbose
fn __verbose__short() -> [Inline; 1] {
	[bold("-v")]
}
fn __verbose__long() -> [Inline; 1] {
	[bold("--verbose")]
}

// rpath
fn __rpath__long() -> [Inline; 3] {
	[bold("--rpath"), roman("="), italic("value")]
}

// pass runner
fn __pass_runner__long() -> [Inline; 2] {
	[bold("--pass-runner"), roman("={integrated|opt}")]
}

// print passes
fn __dump_passes__long() -> [Inline; 1] {
	[bold("--dump-passes")]
}

// passes
fn __passes__short() -> [Inline; 4] {
	[bold("-p"), roman(" "), italic("pass"), roman("[,...]")]
}
fn __passes__long() -> [Inline; 4] {
	[
		bold("--passes"),
		roman("="),
		italic("pass"),
		roman("[,...]"),
	]
}
fn __no_default_passes__long() -> [Inline; 1] {
	[bold("--no-default-passes")]
}

// optimisation level
fn __optimise__short() -> [Inline; 4] {
	[bold("-O"), roman(" ["), italic("level"), roman("]")]
}
fn __optimise__long() -> [Inline; 4] {
	[bold("--optimise"), roman("[="), italic("level"), roman("]")]
}
fn __no_optimise__long() -> [Inline; 1] {
	[bold("--no-optimise")]
}

// symbol stripping
fn __strip__long() -> [Inline; 2] {
	[
		bold("--strip"),
		roman("[={all|unneeded|locals|debug|none}[,...]]"),
	]
}
fn __no_strip__long() -> [Inline; 1] {
	[bold("--no-strip")]
}

// emit
fn __emit__short() -> [Inline; 2] {
	[bold("-e"), roman(" {elf-linked|elf-obj|asm|llvm-ir}")]
}
fn __emit__long() -> [Inline; 2] {
	[bold("--emit"), roman("={elf-linked|elf-obj|asm|llvm-ir}")]
}

// output file
fn __output__short() -> [Inline; 3] {
	[bold("-o"), roman(" "), italic("file")]
}
fn __output__long() -> [Inline; 3] {
	[bold("--output-file"), roman("="), italic("file")]
}

// help + version
fn __help__short() -> [Inline; 1] {
	[bold("-h")]
}
fn __help__long() -> [Inline; 1] {
	[bold("--help")]
}
fn __version__short() -> [Inline; 1] {
	[bold("-V")]
}
fn __version__long() -> [Inline; 1] {
	[bold("--version")]
}

// [-- args...]
fn __args__suffix() -> [Inline; 4] {
	[bold("--"), roman(" "), italic("args"), roman("...")]
}

fn synopsis_section(roff: &mut Roff) {
	macro_rules! option {
		($option:expr, newline = true $(,)?) => {{
			let mut inlines: VecDeque<Inline> = VecDeque::from([line_break(), roman("[")]);
			inlines.extend($option);
			inlines.push_back(roman("]"));
			inlines
		}};
		($option:expr $(, newline = false)? $(,)?) => {{
			let mut inlines: VecDeque<Inline> = VecDeque::from([roman("[")]);
			inlines.extend($option);
			inlines.push_back(roman("]"));
			inlines
		}};
		($vec:expr, $option:expr, mode = OR, newline = true $(,)?) => {{
			let mut inlines: VecDeque<Inline> = $vec;
			inlines.push_front(line_break());
			inlines.pop_back();
			inlines.push_back(roman("|"));
			inlines.extend($option);
			inlines.push_back(roman("]"));
			inlines
		}};
		($vec:expr, $option:expr, mode = OR $(, newline = false)? $(,)?) => {{
			let mut inlines: VecDeque<Inline> = $vec;
			inlines.pop_back();
			inlines.push_back(roman("|"));
			inlines.extend($option);
			inlines.push_back(roman("]"));
			inlines
		}};
		($vec:expr, $option:expr, mode = AND, newline = true $(,)?) => {{
			let mut inlines: VecDeque<Inline> = $vec;
			inlines.push_front(line_break());
			inlines.push_back(roman(" ["));
			inlines.extend($option);
			inlines.push_back(roman("]"));
			inlines
		}};
		($vec:expr, $option:expr, mode = AND, newline = true, wrap = false $(,)?) => {{
			let mut inlines: VecDeque<Inline> = $vec;
			inlines.push_front(line_break());
			inlines.push_back(roman(" "));
			inlines.extend($option);
			inlines
		}};
		($vec:expr, $option:expr, mode = AND, newline = true $(, wrap = true)? $(,)?) => {{
			let mut inlines: VecDeque<Inline> = $vec;
			inlines.push_front(line_break());
			inlines.push_back(roman(" ["));
			inlines.extend($option);
			inlines.push_back(roman("]"));
			inlines
		}};
		($vec:expr, $option:expr, mode = AND $(, newline = false)? , wrap = false $(,)?) => {{
			let mut inlines: VecDeque<Inline> = $vec;
			inlines.push_back(roman(" "));
			inlines.extend($option);
			inlines
		}};
		($vec:expr, $option:expr, mode = AND $(, newline = false)? $(, wrap = true)? $(,)?) => {{
			let mut inlines: VecDeque<Inline> = $vec;
			inlines.push_back(roman(" ["));
			inlines.extend($option);
			inlines.push_back(roman("]"));
			inlines
		}};
	}

	roff.control("SH", ["SYNOPSIS"]);
	roff.control("PP", []);
	roff.text([bold(PROGRAM)]);
	roff.control("RS", []);

	roff.text(option!(
		option!(__jobs__short()),
		__jobs__long(),
		mode = OR,
		newline = true,
	));

	roff.text(option!(__reloc_mode__long(), newline = true));

	roff.text(option!(
		option!(__target__long()),
		__target_native__long(),
		mode = AND,
		newline = true,
	));

	roff.text(option!(
		option!(__verbose__short()),
		__verbose__long(),
		mode = OR,
		newline = true,
	));

	roff.text(option!(__rpath__long(), newline = true));

	roff.text(option!(__pass_runner__long(), newline = true));

	roff.text(option!(
		option!(option!(__passes__short()), __passes__long(), mode = OR),
		option!(
			option!(__dump_passes__long()),
			__no_default_passes__long(),
			mode = AND,
		),
		mode = AND,
		newline = true,
		wrap = false,
	));

	roff.text(option!(
		option!(option!(__optimise__short()), __optimise__long(), mode = OR),
		__no_optimise__long(),
		mode = AND,
		newline = true,
	));

	roff.text(option!(
		option!(__strip__long()),
		__no_strip__long(),
		mode = AND,
		newline = true,
	));

	roff.text(option!(
		option!(__emit__short()),
		__emit__long(),
		mode = OR,
		newline = true,
	));

	roff.text(option!(
		option!(__output__short()),
		__output__long(),
		mode = OR,
		newline = true,
	));

	roff.text(option!(
		option!(option!(__help__short()), __help__long(), mode = OR),
		option!(option!(__version__short()), __version__long(), mode = OR),
		mode = AND,
		newline = true,
		wrap = false,
	));

	// input file(s)
	roff.text([line_break(), italic("file"), roman("...")]);

	roff.text(option!(__args__suffix(), newline = true));

	roff.control("RE", []);
}

fn description_section(roff: &mut Roff) {
	roff.control("SH", ["DESCRIPTION"]);
	roff.control("PP", []);
	roff.text([roman("Foo bar baz.")]);
}

fn options_section(roff: &mut Roff) {
	macro_rules! option {
		($($option:expr),* $(,)?) => {
			roff.control("TP", []);
			roff.text({
				let mut inlines: Vec<Inline> = Vec::with_capacity(9);
				$({
					inlines.extend($option);
					inlines.push(roman(", "));
				};)*
				inlines.pop();
				inlines
			});
			// roff.text($name);
			roff.control("RS", []);
		};
	}
	roff.control("SH", ["OPTIONS"]);
	roff.control("PP", []);
	{
		option!(__jobs__short(), __jobs__long());
		roff.text([
			roman("Specifies the number of parallel jobs to run."),
			line_break(),
			roman("The value of "),
			italic("n"),
			roman(" must be a positive integer."),
		]);
		roff.control("RE", []);
	};
	{
		option!(__reloc_mode__long());
		roff.text([
			roman("Specifies the "),
			bold("code relocation mode"),
			roman(" for compilation."),
			line_break(),
			roman("Possible values:"),
		]);
		dot_point(roff);
		roff.text([
			bold("default"),
			roman(": TODO: look into how LLVM decides the default."),
		]);
		dot_point(roff);
		roff.text([bold("static"), roman(": Static relocation.")]);
		dot_point(roff);
		roff.text([
			bold("pic"),
			roman(": Dynamic relocation, with Position Independent Code."),
		]);
		dot_point(roff);
		roff.text([
			bold("dynamic-no-pic"),
			roman(": Dynamic relocation, without Position Independent Code."),
		]);
		roff.control("RE", []);
	};
	{
		option!(__target__long());
		roff.text([
			roman("Specifies the "),
			bold("target tuple"),
			roman(" (target triple) for compilation."),
			line_break(),
			roman("Target tuples take the form of "),
			bold("<architecture>-<vendor>-linux-<environment>"),
			roman("."),
			line_break(),
			roman("Currently, "),
			bold(PROGRAM),
			roman("'s (officially) supported target tuples are:"),
		]);
		const TRIPLES: [&'static str; 5] = [
			"x86_64-unknown-linux-unknown",
			"aarch64-unknown-linux-unknown",
			"powerpc64-unknown-linux-unknown",
			"riscv64-unknown-linux-unknown",
			"loongarch64-unknown-linux-unknown",
		];
		for triple in TRIPLES {
			dot_point(roff);
			roff.text([bold(triple)]);
		}
		roff.control("PP", []);
		roff.text([
			roman("Users may specify the native values of "),
			bold("<vendor>"),
			roman(" and "),
			bold("<environment>"),
			roman(" for their system at their own discretion, without facing compile errors."),
			line_break(),
			roman("Non-64-bit architectures are not supported."),
			line_break(),
			roman("Non-Linux operating systems are not supported."),
			line_break(),
			line_break(),
			roman("Further reading:"),
		]);
		const FURTHER_READING: [&'static str; 3] = [
			"https://mcyoung.xyz/2025/04/14/target-triples/",
			"https://llvm.org/docs/LangRef.html#target-triple",
			"https://llvm.org/doxygen/classllvm_1_1Triple.html#details",
		];
		for src in FURTHER_READING {
			dot_point(roff);
			roff.control("UR", [src]);
			roff.control("UE", []);
		}
		roff.control("RE", []);
	};
	{
		option!(__target_native__long());
		roff.text([
			roman("Sets the "),
			bold("target tuple"),
			roman(" to the system's native target tuple."),
			line_break(),
			roman("This is effectively equivalent to specifying "),
			bold("--target=\"$(clang --print-target-triple)\""),
			roman("."),
		]);
		roff.control("RE", []);
	};
	{
		option!(__verbose__short(), __verbose__long());
		roff.text([roman("Enables verbose debug printing.")]);
		roff.control("RE", []);
	};
	{
		option!(__rpath__long());
		roff.text([
			roman("Specifies the "),
			bold("RUNPATH"),
			roman(" for the outputted dynamically linked elf file."),
			line_break(),
			roman("The value should consist of colon-delimited directories for "),
			bold("ld.so"),
			roman("(8) to look for libraries in."),
		]);
		roff.control("RE", []);
	};
	{
		option!(__pass_runner__long());
		roff.text([
			roman("Specifies how LLVM optimisation & analysis passes should be ran."),
			line_break(),
			roman("Possible values:"),
		]);
		{
			dot_point(roff);
			roff.text([
				bold("integrated"),
				roman(": uses libLLVM to run passes in-process."),
			]);
			dot_point(roff);
			roff.text([
				bold("opt"),
				roman(": Uses "),
				bold("opt"),
				roman("(1) to run passes externally."),
			]);
		};
		roff.control("RE", []);
	};
	{
		option!(__passes__short(), __passes__long());
		roff.text([
			roman("Specifies the LLVM optimisation & analysis passes to run."),
			line_break(),
			roman("This argument may be specified multiple times."),
		]);
		roff.control("RE", []);
	};
	{
		option!(__dump_passes__long());
		roff.text([roman(
			"Prints all available LLVM optimisation & analysis passes, then terminates.",
		)]);
		roff.control("RE", []);
	};
	{
		option!(__no_default_passes__long());
		roff.text([roman(
			"Disables automatic use of default LLVM optimisation & analysis passes.",
		)]);
		roff.control("RE", []);
	};
	{
		option!(__optimise__short(), __optimise__long());
		roff.text([
			roman("Specifies the "),
			bold("optimisation level"),
			roman(" for compilation."),
			line_break(),
			roman("Omitting "),
			italic("level"),
			roman(" is equivalent to specifying "),
			bold("-O3"),
			roman("."),
		]);
		roff.control("RE", []);
	};
	{
		option!(__no_optimise__long());
		roff.text([
			roman("Sets the "),
			bold("optimisation level"),
			roman(" to zero (disables optimisations)."),
			line_break(),
			roman("This is equivalent to specifying "),
			bold("-O0"),
			roman("."),
		]);
		roff.control("RE", []);
	};
	{
		option!(__help__short(), __help__long());
		roff.text([roman("Opens this man page.")]);
		roff.control("RE", []);
	};
	{
		option!(__version__short(), __version__long());
		roff.text([roman("Prints version information, then terminates.")]);
		roff.control("RE", []);
	};
}

fn environment_section(roff: &mut Roff) {
	roff.control("SH", ["ENVIRONMENT"]);
	simple_env_var(roff, "CFLAGS", "args", true);
	simple_env_var(roff, "LLVM_PASSES", "--passes", false);
	command_env_var(
		roff,
		"LD",
		"ld",
		"1",
		&[("ld.bfd", "1"), ("ld.lld", "1"), ("ld.mold", "1")],
	);
	alias_env_var(roff, "LINKER", "LD");
	command_env_var(roff, "STRIP", "strip", "1", &[("llvm-strip", "1")]);
	command_env_var(roff, "OPT", "opt", "1", &[]);
}

fn alias_env_var(roff: &mut Roff, name: &str, actual: &str) {
	roff.control("TP", []);
	roff.text([bold(name)]);
	roff.control("RS", []);
	roff.text([
		roman("An alias for $"),
		bold(actual),
		roman("."),
		line_break(),
		roman("If $"),
		bold(actual),
		roman(" is set, $"),
		bold(name),
		roman(" is ignored."),
	]);
	roff.control("RE", []);
}

fn simple_env_var(roff: &mut Roff, name: &str, cli: &str, is_positional: bool) {
	roff.control("TP", []);
	roff.text([bold(name)]);
	roff.control("RS", []);
	roff.text([
		roman("Setting $"),
		bold(name),
		roman(" shall have the same effect as setting "),
		if is_positional {
			italic(cli)
		} else {
			bold(cli)
		},
		roman(" to the value of $"),
		bold(name),
		roman("."),
	]);
	roff.control("RE", []);
}

fn command_env_var(
	roff: &mut Roff,
	name: &str,
	command: &str,
	section: &str,
	good_alternatives: &[(&str, &str)],
) {
	roff.control("TP", []);
	roff.text([bold(name)]);
	roff.control("RS", []);
	let mut text: Vec<Inline> = vec![
		roman("Setting $"),
		bold(name),
		roman(" shall cause "),
		bold(PROGRAM),
		roman(" to execute the value of $"),
		bold(name),
		roman(" as a command in "),
		bold(command),
		roman("("),
		roman(section),
		roman(")'s stead."),
	];
	if !good_alternatives.is_empty() {
		text.push(line_break());
		text.push(roman("Known compatible programs:"));
	};
	roff.text(text);
	for (alt_name, alt_section) in good_alternatives {
		dot_point(roff);
		man_ref(roff, &[(alt_name, alt_section)])
	}
	roff.control("RE", []);
}

fn see_also_section(roff: &mut Roff) {
	roff.control("SH", ["SEE ALSO"]);
	roff.control("PP", []);
	man_ref(
		roff,
		&[
			("clang", "1"),
			("ld.so", "8"),
			("ld", "1"),
			("ld.bfd", "1"),
			("ld.gold", "1"),
			("ld.lld", "1"),
			("ld.mold", "1"),
			("ld.wild", "1"),
		],
	);
}
