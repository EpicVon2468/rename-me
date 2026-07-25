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
			"2026-07-25",
			concat!("rename-me ", env!("CARGO_PKG_VERSION")),
		],
	);
	name_section(&mut page);
	synopsis_section(&mut page);
	description_section(&mut page);
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

macro_rules! man_ref {
	($roff:expr; $name:expr => $section:expr $(, $($name_extra:expr => $section_extra:expr),*)? $(,)?) => {
		$roff.text([
			bold($name),
			roman(format!("({})", $section)),
			$($(
				roman(", "),
				bold($name_extra),
				roman(format!("({})", $section_extra)),
			)*)?
		])
	};
}

fn section<'roff>(roff: &'roff mut Roff, name: &'roff str) -> &'roff mut Roff {
	roff.control("SH", [name])
}

fn name_section(roff: &mut Roff) {
	section(roff, "NAME").text([roman(format!("{PROGRAM} - the rename-me compiler"))]);
}

fn synopsis_section(roff: &mut Roff) {
	section(roff, "SYNOPSIS").text([bold(PROGRAM)]);
	roff.control("RS", []);
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
			roman("={integrated|llvm-opt}]"),
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
	section(roff, "DESCRIPTION");
	roff.text([roman("Foo bar baz.")]);
}

fn environment_section(roff: &mut Roff) {
	section(roff, "ENVIRONMENT");
	simple_env_var(roff, "CFLAGS", "args", true);
	simple_env_var(roff, "LLVM_PASSES", "--passes", false);
	command_env_var(
		roff,
		"LD",
		"ld",
		"1",
		vec![("ld.bfd", "1"), ("ld.lld", "1"), ("ld.mold", "1")],
	);
	alias_env_var(roff, "LINKER", "LD");
	command_env_var(roff, "STRIP", "strip", "1", vec![("llvm-strip", "1")]);
	command_env_var(roff, "OPT", "opt", "1", vec![]);
}

fn alias_env_var(roff: &mut Roff, name: &str, actual: &str) {
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
	good_alternatives: Vec<(&str, &str)>,
) {
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
		roff.text([bold(alt_name), roman("("), roman(alt_section), roman(")")]);
	}
	roff.text([line_break()]);
	roff.control("RE", []);
}

fn see_also_section(roff: &mut Roff) {
	section(roff, "SEE ALSO");
	man_ref!(
		roff;
		"clang" => "1",
		"ld.so" => "8",
		"ld" => "1",
		"ld.bfd" => "1",
		"ld.gold" => "1",
		"ld.lld" => "1",
		"ld.mold" => "1",
		"ld.wild" => "1",
	);
}
