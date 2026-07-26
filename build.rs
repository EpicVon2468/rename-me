use std::{env::var as env, fs::File};

use roff::{Inline, Roff, bold, italic, line_break, roman};

const PROGRAM: &'static str = "renamec";

pub fn main() {
	let mut page: Roff = Roff::new();
	page.control(
		"TH",
		[
			"RENAMEC",
			"1",
			"2026-07-26",
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

fn name_section(roff: &mut Roff) {
	roff.control("SH", ["NAME"]);
	roff.control("PP", []);
	roff.text([roman(format!("{PROGRAM} – the rename-me compiler"))]);
}

fn synopsis_section(roff: &mut Roff) {
	roff.control("SH", ["SYNOPSIS"]);
	roff.control("PP", []);
	roff.text([bold(PROGRAM)]);
	roff.control("RS", []);
	{
		// target triple
		roff.text([
			line_break(),
			roman("["),
			bold("--target"),
			roman("="),
			italic("triple"),
			roman("] ["),
			bold("--native-target"),
			roman("]"),
		]);
	};
	{
		// verbose
		roff.text([
			line_break(),
			roman("["),
			bold("-v"),
			roman("|"),
			bold("--verbose"),
			roman("]"),
		]);
	};
	{
		// RPATH
		roff.text([
			line_break(),
			roman("["),
			bold("--rpath"),
			roman("="),
			italic("value"),
			roman("]"),
		]);
	};
	{
		// pass runner
		roff.text([
			line_break(),
			roman("["),
			bold("--pass-runner"),
			roman("={integrated|opt}]"),
		]);
	};
	{
		// passes
		roff.text([
			line_break(),
			roman("["),
			bold("-p"),
			roman(" "),
			italic("pass"),
			roman("[,...]|"),
			bold("--passes"),
			roman("="),
			italic("pass"),
			roman("[,...]] ["),
			bold("--no-default-passes"),
			roman("]"),
		]);
	};
	{
		// optimisation level
		roff.text([
			line_break(),
			roman("["),
			bold("-O"),
			roman(" ["),
			italic("level"),
			roman("]|"),
			bold("--optimise"),
			roman("[="),
			italic("level"),
			roman("]] ["),
			bold("--no-optimise"),
			roman("]"),
		]);
	};
	{
		// symbol stripping
		roff.text([
			line_break(),
			roman("["),
			bold("--strip"),
			roman("[={all|unneeded|locals|debug|none}[,...]]] ["),
			bold("--no-strip"),
			roman("]"),
		]);
	};
	{
		// emit
		roff.text([
			line_break(),
			roman("["),
			bold("-e"),
			roman(" {elf-linked|elf-obj|asm|llvm-ir}|"),
			bold("--emit"),
			roman("={elf-linked|elf-obj|asm|llvm-ir}"),
			roman("]"),
		]);
	};
	{
		// output file
		roff.text([
			line_break(),
			roman("["),
			bold("-o"),
			roman(" "),
			italic("file"),
			roman("|"),
			bold("--output-file"),
			roman("="),
			italic("file"),
			roman("]"),
		]);
	};
	{
		// help + version
		roff.text([
			line_break(),
			roman("["),
			bold("--help"),
			roman("] ["),
			bold("-V"),
			roman("|"),
			bold("--version"),
			roman("]"),
		]);
	};
	{
		// input file(s)
		roff.text([line_break(), italic("file"), roman("...")]);
	};
	{
		// args
		roff.text([
			line_break(),
			roman("["),
			bold("--"),
			roman(" "),
			italic("args"),
			roman("...]"),
		]);
	};
	roff.control("RE", []);
}

fn description_section(roff: &mut Roff) {
	roff.control("SH", ["DESCRIPTION"]);
	roff.control("PP", []);
	roff.text([roman("Foo bar baz.")]);
}

fn options_section(roff: &mut Roff) {
	macro_rules! option {
		($name:expr $(,)?) => {
			roff.control("TP", []);
			roff.text($name);
			roff.control("RS", []);
		};
	}
	roff.control("SH", ["OPTIONS"]);
	roff.control("PP", []);
	{
		option!([bold("--target"), roman("="), italic("triple")]);
		roff.text([
			roman("Specifies the "),
			bold("target triple"),
			roman(" for compilation."),
			line_break(),
			roman("Target triples take the form of "),
			bold("<architecture>-<vendor>-linux-<environment>"),
			roman("."),
			line_break(),
			roman("Currently, "),
			bold(PROGRAM),
			roman("'s (officially) supported target triples are:"),
		]);
		const TRIPLES: [&'static str; 5] = [
			"x86_64-unknown-linux-unknown",
			"aarch64-unknown-linux-unknown",
			"powerpc64-unknown-linux-unknown",
			"riscv64-unknown-linux-unknown",
			"loongarch64-unknown-linux-unknown",
		];
		for triple in TRIPLES {
			roff.control("IP", ["\\(bu", "2"]);
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
			roman("Further reading materials:"),
		]);
		const FURTHER_READING: [&'static str; 3] = [
			"https://mcyoung.xyz/2025/04/14/target-triples/",
			"https://llvm.org/docs/LangRef.html#target-triple",
			"https://llvm.org/doxygen/classllvm_1_1Triple.html#details",
		];
		for src in FURTHER_READING {
			roff.control("IP", ["\\(bu", "2"]);
			roff.control("UR", [src]);
			roff.control("UE", []);
		}
		roff.control("RE", []);
	};
	{
		option!([bold("--native-target")]);
		roff.text([
			roman("Sets the "),
			bold("target triple"),
			roman(" to the system's native target triple."),
			line_break(),
			roman("This is effectively equivalent to specifying "),
			bold("--target=\"$(clang --print-target-triple)\""),
			roman("."),
		]);
		roff.control("RE", []);
	};
	{
		option!([bold("-v"), roman(", "), bold("--verbose")]);
		roff.text([roman("Enables verbose debug printing.")]);
		roff.control("RE", []);
	};
	{
		option!([bold("--rpath"), roman("="), italic("value")]);
		roff.text([
			roman("Sets the "),
			bold("RPATH"),
			roman(" for the outputted dynamically linked elf file."),
			line_break(),
			roman("The value should consist of colon-delimited directories for "),
		]);
		man_ref(roff, &[("ld.so", "8")]);
		roff.text([roman("to look for libraries in.")]);
		roff.control("RE", []);
	};
	{
		option!([bold("--pass-runner"), roman("={integrated|opt}")]);
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
		line_break(),
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
		line_break(),
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
		roff.control("IP", ["\\(bu", "2"]);
		man_ref(roff, &[(alt_name, alt_section)])
	}
	roff.text([line_break()]);
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
