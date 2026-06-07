use std::hint::{cold_path, unreachable_unchecked};
use std::io::BufRead;

use anyhow::{Context as _, Result, bail};

/// `LETTER : [a-zA-Z]`
///
/// `NUM : [0-9]`
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Token {
	/// `IDENTIFIER : '_'* LETTER+ ( '_' | LETTER )* ;`
	Identifier(String),
	/// Integer value.
	///
	/// `LITERAL : MINUS? NUM+ ( '_' NUM+ )* ;`
	Literal(String),
	/// Floating-point value.
	///
	/// `REAL : MINUS? NUM+ ( '_' NUM+ )* ( '.' NUM+ ( '_' NUM+ )* )? 'f' ;`
	Real(String),
	/// `FUNCTION : 'f' 'u' 'n' 'c' 't' ;`
	Function,
	/// `UNSAFE : 'u' 'n' 's' 'a' 'f' 'e' ;`
	Unsafe,
	/// `EXTERNAL : 'e' 'x' 't' 'e' 'r' 'n' ;`
	External,
	/// `CONSTANT : 'c' 'o' 'n' 's' 't' ;`
	Constant,
	/// `RETURN : 'r' 'e' 't' 'u' 'r' 'n' ;`
	Return,
	/// `VAL : 'v' 'a' 'l' ;`
	Val,
	/// `OPEN_BRACE : '{' ;`
	OpenBrace,
	/// `CLOSE_BRACE : '}' ;`
	CloseBrace,
	/// `OPEN_BRACKET : '(' ;`
	OpenBracket,
	/// `CLOSE_BRACKET : ')' ;`
	CloseBracket,
	/// `OPEN_SQUARE_BRACKET : '[' ;`
	OpenSquareBracket,
	/// `CLOSE_SQUARE_BRACKET : ']' ;`
	CloseSquareBracket,
	/// `SEMICOLON : ';' ;`
	Semicolon,
	/// `COLON : ':' ;`
	Colon,
	/// Div.
	///
	/// `SLASH : '/' ;`
	Slash,
	/// Mul or ptr.
	///
	/// `ASTERISK : '*' ;`
	Asterisk,
	/// `PLUS : '+' ;`
	Plus,
	/// (Unary) minus.
	///
	/// `MINUS : '-' ;`
	Minus,
	/// Borrow or 'boolean and'.
	///
	/// `AMPERSAND : '&' ;`
	Ampersand,
	/// `COMMA : ',' ;`
	Comma,
	/// `EQUALS : '=' ;`
	Equals,
}

pub type Src<'a> = &'a mut dyn BufRead;

pub struct Lexer<'a> {
	// TODO: use `BufRead::fill_buf()` to 'peek' chars.
	src: Src<'a>,
	cached: Option<Token>,
}

impl<'a> Lexer<'a> {
	pub fn new(src: Src<'a>) -> Self {
		Self { src, cached: None }
	}

	#[expect(clippy::panic_in_result_fn)]
	pub fn peek_token(&mut self) -> Result<&Token> {
		assert_eq!(self.cached, None);
		let result: Token = self.read_token()?;
		Ok(self.cached.insert(result))
	}

	pub fn read_token(&mut self) -> Result<Token> {
		if let Some(token) = self.cached.take() {
			return Ok(token);
		};
		let first: char = self.read_until_substantial_char()?;
		Ok(match first {
			'{' => Token::OpenBrace,
			'}' => Token::CloseBrace,
			'(' => Token::OpenBracket,
			')' => Token::CloseBracket,
			'[' => Token::OpenSquareBracket,
			']' => Token::CloseSquareBracket,
			';' => Token::Semicolon,
			':' => Token::Colon,
			'/' => Token::Slash,
			'*' => Token::Asterisk,
			'+' => Token::Plus,
			'-' => Token::Minus,
			'&' => Token::Ampersand,
			',' => Token::Comma,
			'=' => Token::Equals,
			_ => bail!(""),
		})
	}

	pub fn read_until_substantial_char(&mut self) -> Result<char> {
		let mut current: char = self.read_char()?;
		while current.is_whitespace() {
			current = self.read_char()?;
		}
		Ok(current)
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
