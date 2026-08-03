#![feature(exit_status_error)]

use std::{
	env::{var, var_os},
	ffi::{OsStr, OsString},
	process::{Command, Output, Stdio},
};

fn llvm_config(args: &[impl AsRef<OsStr>]) -> Vec<String> {
	let llvm_config: OsString = match var_os("LLVM_PREFIX") {
		Some(mut prefix) => {
			prefix.push("/bin/llvm-config");
			prefix
		},
		None => OsString::from("llvm-config"),
	};

	let mut command: Command = Command::new(llvm_config);
	command.arg(cfg_select! {
		any(feature = "prefer-static", feature = "force-static") => "--link-static",
		_ => "--link-shared",
	});
	for arg in args {
		command.arg(arg);
	}
	let output: Output = command
		.stdout(Stdio::piped())
		.spawn()
		.unwrap()
		.wait_with_output()
		.unwrap()
		.exit_ok()
		.unwrap();
	shell_words::split(&str::from_utf8(&output.stdout).unwrap()).unwrap()
}

fn system_lib_dirs() -> Vec<String> {
	let gcc_target_tuple: String = format!("{}-linux-gnu", var("CARGO_CFG_TARGET_ARCH").unwrap());
	vec![
		"/lib".to_owned(),
		"/lib64".to_owned(),
		format!("/lib/{gcc_target_tuple}"),
		"/usr/lib".to_owned(),
		"/usr/lib64".to_owned(),
		format!("/usr/lib/{gcc_target_tuple}"),
		"/usr/local/lib".to_owned(),
		"/usr/local/lib64".to_owned(),
		format!("/usr/local/lib/{gcc_target_tuple}"),
	]
}

pub fn main() {
	println!("cargo::rerun-if-env-changed=LLVM_PREFIX");
	if let Ok(path) = var("LLVM_PREFIX") {
		println!("cargo::rerun-if-changed={path}");
	};

	cc::Build::new()
		.warnings(false)
		.extra_warnings(false)
		.inherit_rustflags(false)
		.file("wrappers/target.c")
		.flags(llvm_config(&["--cflags"]))
		.compile("targetwrappers");

	let [libdir]: &[String] = &llvm_config(&["--libdir"]) else {
		unreachable!();
	};

	// Export information to other crates
	// println!("cargo:config_path={}", llvm_config_path.display()); // DEP_LLVM_CONFIG_PATH
	// Works with 'cargo:', breaks with 'cargo::'
	println!("cargo:libdir={libdir}"); // DEP_LLVM_LIBDIR

	// Link LLVM libraries
	println!("cargo::rustc-link-search=native={libdir}");
	for path in system_lib_dirs() {
		println!("cargo::rustc-link-search=native={path}");
	}
	for name in llvm_config(&["--libnames"]) {
		println!(
			concat!(
				"cargo::rustc-link-lib=",
				cfg_select! {
					any(feature = "prefer-static", feature = "force-static") => "static",
					_ => "dylib",
				},
				"={}",
			),
			// Strip 'lib' prefix, and '.a' / '.so' suffix.
			&name[3..name.rfind('.').unwrap()],
		);
	}

	// Link system libraries
	// We get the system libraries based on the kind of LLVM libraries we link to, but we link to
	// system libs based on the target environment.
	let mut system_libs: Vec<String> = llvm_config(&["--system-libs"]);
	system_libs.push("-lffi".to_owned());
	for name in system_libs {
		println!(
			concat!(
				"cargo::rustc-link-lib=",
				cfg_select! {
					target_feature = "crt-static" => "static",
					_ => "dylib",
				},
				"={}",
			),
			// Strip '-l' prefix.
			&name[2..],
		);
	}
}
