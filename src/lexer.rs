use anyhow::{Context as _, Result, bail};
use std::hint::{cold_path, unreachable_unchecked};
use std::io::Read;

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Token {
	/// `[a-zA-Z]+`
	Identifier(String),
	/// `(\+)?[0-9]+((_[0-9]+)+)?`
	Literal(String),
	/// `((\+)?[0-9]+((_[0-9]+)+)?)((\.((\+)?[0-9]+((_[0-9]+)+)?)f)|f)`
	Real(String),
	/// `funct`
	Function,
	/// `unsafe`
	Unsafe,
	/// `return`
	Return,
	/// `{`
	OpenBrace,
	/// `}`
	CloseBrace,
	/// `(`
	OpenParen,
	/// `)`
	CloseParen,
	/// `;`
	Semicolon,
	/// `:`
	Colon,
	/// `/`
	Div,
	// mul or ptr
	/// `*`
	Asterisk,
	/// `+`
	Add,
	// subtract or unary minus
	/// `-`
	Minus,
}

pub struct Lexer<'a> {
	src: &'a mut dyn Read,
}

impl<'a> Lexer<'a> {
	pub fn new(src: &'a mut dyn Read) -> Self {
		Self { src }
	}

	pub fn read_char(&mut self) -> Result<char> {
		let [initial]: [u8; 1] = {
			let mut buf: [u8; 1] = [0; 1];
			self.src.read_exact(&mut buf)?;
			buf
		};
		// Determine UTF-8 byte length for a multibyte char.
		let len: usize = match initial {
			_ if (initial & 0b1111_1000) == 0b1111_0000 => 4,
			_ if (initial & 0b1111_0000) == 0b1110_0000 => 3,
			_ if (initial & 0b111_00000) == 0b110_00000 => 2,
			// SANITY: Fast path, single-byte char.
			_ => return Ok(char::from(initial)),
		};
		let mut buf: Vec<u8> = vec![0; len];
		buf[0] = initial;
		self.src.read_exact(&mut buf[1..])?;
		let collected: &str = str::from_utf8(&buf).context("Invalid char input!")?;
		let str_len: usize = collected.len();
		if str_len != len {
			// TODO: Is this statement fully unreachable, or can invalid sequences cause it?
			// SANITY: The code above should make this path impossible to reach.
			cold_path();
			bail!("Expected str to have length '{len}', but instead got length '{str_len}'!");
		};
		let Some(result): Option<char> = collected.chars().next() else {
			// SANITY: The above length comparison check means that this cannot be reached.
			// SAFETY:
			// Problem(s):
			// - `unreachable_unchecked()` is unsafe, and it is Undefined Behaviour for it to be reached.
			// Excuse(s):
			// - This statement cannot be reached.
			unsafe {
				unreachable_unchecked();
			};
		};
		Ok(result)
	}

	pub const fn read_token(&mut self) -> Option<Token> {
		None
	}
}
