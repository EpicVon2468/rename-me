use derive_more::{Constructor, Display};

use thiserror::Error;

#[derive(Error, Debug)]
#[derive_const(Constructor)]
#[error("An Internal Compiler Error (ICE) occurred whilst {0}!  This was not intentional, and is a bug in the compiler!")]
pub struct ICE(ErrorSource);

#[derive(Debug, Display)]
pub enum ErrorSource {
	#[display("lexing input")]
	Lexing,
	#[display("parsing tokens")]
	Parsing,
	#[display("generating output")]
	CodeGen,
}
