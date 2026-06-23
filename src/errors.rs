use std::io::Error as IOError;

use derive_more::{Constructor, Display};

use thiserror::Error;

use crate::lexer::Token;

#[must_use]
#[derive(Debug, Error)]
#[derive_const(Constructor)]
#[error(
	"An Internal Compiler Error occurred whilst compiling {0}!  This is a compiler bug, not a bug your code!"
)]
pub struct ICE(ErrorSource);

#[must_use]
#[derive(Debug, Error)]
#[derive_const(Constructor)]
#[error("An error occurred whilst compiling {0}!")]
pub struct CompileError(ErrorSource);

#[derive(Debug, Display)]
#[derive_const(Constructor)]
#[display("{input_file_name} during phase {phase:?}")]
pub struct ErrorSource {
	input_file_name: &'static str,
	phase: Phase,
}

#[derive(Debug, Display)]
pub enum Phase {
	#[display("lexing input")]
	Lexing,
	#[display("parsing tokens")]
	Parsing,
	#[display("generating output")]
	CodeGen,
}

#[must_use]
#[derive(Debug, Error)]
pub enum LexerError {
	#[error("An I/O error occurred whilst trying to read bytes from source!")]
	ForwardedIOError(#[from] IOError),
	#[error("Disallowed ASCII character appeared!  Character (in decimal) was: '{0}'!")]
	DisallowedASCIIChar(u8),
	#[error("Invalid UTF-8 byte with length of 0 appeared!  Byte (in decimal) was: '{0}'!")]
	ZeroLengthByte(u8),
	#[error("Invalid UTF-8 byte sequence!  Sequence (in decimal(s)) was: {0:?}!")]
	InvalidByteSequence(Vec<u8>),
	#[error("Parsed invalid identifier {}{identifier:?}!", if *is_identifier_start { "start character " } else { "" })]
	InvalidIdentifier {
		identifier: String,
		is_identifier_start: bool,
	},
}

impl LexerError {
	pub const fn invalid_identifier(identifier: String, is_identifier_start: bool) -> Self {
		Self::InvalidIdentifier {
			identifier,
			is_identifier_start,
		}
	}
}

#[must_use]
#[derive(Debug, Error)]
#[derive_const(Constructor)]
#[error("Unexpected token reached!  Actual token was: `{unexpected}`")]
pub struct UnexpectedTokenError {
	// TODO: `expected: Option<&[Token]>,`
	unexpected: Token,
}
