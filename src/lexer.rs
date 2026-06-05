use std::hint::{cold_path, unreachable_unchecked};
use std::io::Read;

use anyhow::{Context as _, Result, bail};

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Token {
	/// `_*[a-zA-Z]+(_|[a-zA-Z])*`
	Identifier(String),
	/// Integer value.
	///
	/// `(\+)?[0-9]+((_[0-9]+)+)?`
	Literal(String),
	/// Floating-point value.
	///
	/// `((\+)?[0-9]+((_[0-9]+)+)?)((\.((\+)?[0-9]+((_[0-9]+)+)?)f)|f)`
	Real(String),
	/// `funct`
	Function,
	/// `unsafe`
	Unsafe,
	/// `extern`
	External,
	/// `return`
	Return,
	/// `{`
	OpenBrace,
	/// `}`
	CloseBrace,
	/// `(`
	OpenBracket,
	/// `)`
	CloseBracket,
	/// `[`
	OpenSqrBracket,
	/// `]`
	CloseSqrBracket,
	/// `;`
	Semicolon,
	/// `:`
	Colon,
	/// `/`
	Div,
	/// Mul or ptr.
	///
	/// `*`
	Asterisk,
	/// `+`
	Add,
	/// Sub or unary minus.
	///
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

	pub const fn read_token(&mut self) -> Option<Token> {
		None
	}

	fn read(&mut self, buf: &mut [u8]) -> Result<()> {
		self.src
			.read_exact(buf)
			.context("An error occurred trying to read bytes (Lexer)!")
	}

	pub fn read_char(&mut self) -> Result<char> {
		let [initial]: [u8; 1] = {
			let mut buf: [u8; 1] = [0; 1];
			self.read(&mut buf)?;
			buf
		};

		if initial == 0 {
			// SANITY(unexpected): Aside from malicious or invalid input, it isn't expected for the NUL byte to appear.
			cold_path();
			bail!("Unexpected NUL byte!");
		};
		// SANITY(fast-path): All ASCII chars are single-byte.
		if initial < 128 {
			return Ok(char::from(initial));
		};

		#[expect(
			clippy::as_conversions,
			reason = "False positive.  These casts are safe."
		)]
		let len: usize = {
			// SANITY(unusual):
			// 128 is subtracted because the value is always >= 128 at this point.
			// This means space can be saved by stripping the first 128 entries.
			// SAFETY:
			// Problem(s):
			// - `u8::unchecked_sub()` can underflow or overflow.
			// Excuse(s):
			// - The amount being subtracted is a trusted constant value (128).
			// - The value of `initial` is always >= 128 at this point, meaning subtracting 128 is always safe.
			let index: usize = unsafe { initial.unchecked_sub(128) } as usize;

			// Determine UTF-8 byte length for a multibyte char.
			// SANITY(unusual): Cheaper to store 128 `u8`s and convert at use-site than to store 128 `usize`s.
			let len: usize = UTF8_CHAR_WIDTH[index] as usize;
			if len == 0 {
				bail!("Invalid byte!");
			};
			len
		};

		let mut buf: Vec<u8> = vec![0; len];
		buf[0] = initial;
		self.read(&mut buf[1..])?;

		// SANITY(unusual): There doesn't appear to be any other way to make a `char` from a `u8` slice than this.
		let string: &str = str::from_utf8(&buf).context("Invalid UTF-8 byte sequence!")?;
		let Some(result): Option<char> = string.chars().next() else {
			// SANITY(unreachable): If `len` was 0, the enclosing function would've `bail!()`'d before this point.
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
}

// SANITY(overhead + unusual):
// Copied from `core`'s str internals, w/o the first 128 entries.
// Cheaper to store 128 `u8`s and convert at use-site than to store 128 `usize`s.
// The first 128 entries (which were all 1) were stripped out, as the only use of this value is non-ASCII.
// FIXME(verbosity): Replace with range check?
// https://tools.ietf.org/html/rfc3629
static UTF8_CHAR_WIDTH: [u8; 128] = [
	// 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // A
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // B
	0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
	2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
	3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
	4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
];
