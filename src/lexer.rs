use std::collections::VecDeque;
use std::hint::{cold_path, unreachable_unchecked};
use std::io::BufRead;

use anyhow::{Context as _, Result, bail};

/// `LETTER : [a-zA-Z]`
///
/// `NUM : [0-9]`
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Token {
	// Can't be only '_', must also consist of at least one letter.
	/// `IDENTIFIER : IDENTIFIER_START IDENTIFIER_REST* ;`
	///
	/// `IDENTIFIER_START : LETTER | '_' ;`
	///
	/// `IDENTIFIER_REST : LETTER | NUM | '_' ;`
	Identifier(String),
	/// Integer value.
	///
	/// `LITERAL : NUM+ ( '_' NUM+ )* ;`
	Literal(String),
	/// Floating-point value.
	///
	/// `REAL : NUM+ ( '_' NUM+ )* ( '.' NUM+ ( '_' NUM+ )* )? 'f' ;`
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
	/// `EXCLAMATION_MARK : '!' ;`
	ExclamationMark,
}

pub type Src<'a> = &'a mut dyn BufRead;

pub struct Lexer<'a> {
	src: Src<'a>,
	cached: Option<Token>,
	cached_chars: VecDeque<char>,
}

macro_rules! expect_char {
	($actual:expr; $($expected:expr),* $(,)?) => {
		$(expect_char!($actual, $expected);)*
	};
	($actual:expr, $expected:expr $(,)?) => {{
		let actual = $actual;
		let expected = $expected;
		if actual != expected {
			bail!("Unexpected character reached, expected '{expected}', got '{actual}'!");
		};
	}};
}

#[allow(clippy::inline_always)]
#[inline(always)]
const fn is_valid_for_identifier_first(input: char) -> bool {
	input.is_ascii_alphabetic() || input == '_'
}

#[allow(clippy::inline_always)]
#[inline(always)]
const fn is_valid_for_identifier_rest(input: char) -> bool {
	input.is_ascii_alphanumeric() || input == '_'
}

impl<'a> Lexer<'a> {
	pub fn new(src: Src<'a>) -> Self {
		Self {
			src,
			cached: None,
			cached_chars: VecDeque::new(),
		}
	}

	pub fn peek_token(&mut self) -> Result<&Token> {
		// SANITY(unusual):
		// This check is here to prevent performing `Option::take()` only to immediately `Option::insert()` the same value.
		// This does mean it is impossible peek further than one token forward.  This behaviour is intentional.
		if let Some(ref value) = self.cached {
			return Ok(value);
		};
		let result: Token = self.read_token()?;
		Ok(self.cached.insert(result))
	}

	pub fn read_token(&mut self) -> Result<Token> {
		if let Some(token) = self.cached.take() {
			return Ok(token);
		};
		let first: char = self.next_sig_char()?;
		Ok(match first {
			_ if is_valid_for_identifier_first(first) => {
				// Restore first to stack so `Self::read_ident_chars()` will eat it.
				self.cached_chars.push_back(first);
				match self.read_ident_chars()?.as_str() {
					"funct" => Token::Function,
					"unsafe" => Token::Unsafe,
					"extern" => Token::External,
					"const" => Token::Constant,
					"return" => Token::Return,
					"val" => Token::Val,
					identifier => Token::Identifier(identifier.to_owned()),
				}
			},
			'0'..='9' => {
				// Restore first to stack so `Self::read_num_chars()` will eat it.
				self.cached_chars.push_back(first);
				let value: String = self.read_num_chars()?;
				if value.ends_with('f') {
					Token::Real(value)
				} else {
					Token::Literal(value)
				}
			},
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
			'!' => Token::ExclamationMark,
			other => bail!("Unrecognised character input '{other}'!"),
		})
	}

	fn read_num_chars(&mut self) -> Result<String> {
		let mut buf: String = String::with_capacity(8);
		let (mut invalid, mut seen_period): (bool, bool) = (true, false);
		let mut current: char = self.read_char()?;
		if !current.is_ascii_digit() {
			bail!("Invalid number start character '{current}'!")
		};
		// TODO: '_' support for numbers
		while current.is_ascii_digit() || current == '.' {
			buf.push(current);
			if current == '.' {
				if seen_period {
					invalid = true;
					break;
				};
				seen_period = true;
			};
			invalid = false;
			current = self.read_char()?;
		}
		if invalid {
			bail!("Invalid number '{buf}'!");
		};
		if seen_period {
			expect_char!(current, 'f');
		};
		if current == 'f' {
			static NEXT_VALID_VALUES: &[char] = &[';', '}', '-', '+', '*', '/', '&', '|'];

			buf.push(current);
			let next: char = self.read_char()?;
			let is_next_significant: bool = !next.is_whitespace();

			if !NEXT_VALID_VALUES.contains(&next) && is_next_significant {
				bail!("Unexpected character(s) found after real '{buf}'!  First was '{next}'!")
			};
			if is_next_significant {
				self.cached_chars.push_back(next);
			};
		};
		Ok(buf)
	}

	fn read_ident_chars(&mut self) -> Result<String> {
		let mut buf: String = String::with_capacity(16);
		let mut invalid: bool = true;
		let mut current: char = self.read_char()?;
		if !is_valid_for_identifier_first(current) {
			bail!("Invalid identifier start character '{current}'!");
		};
		while is_valid_for_identifier_rest(current) {
			// SANITY(unusual + verbosity):
			// This expression short-circuits to prevent checking `current` if `invalid` is already false.
			if invalid && current.is_ascii_alphabetic() {
				// There needs to be at least one alphabetic character in an identifier.
				invalid = false;
			};
			buf.push(current);
			current = self.read_char()?;
		}
		// SANITY(unchecked):
		// No check is performed to determine whether `buf` is empty.
		// The `invalid` variable is true by default, the only way for it to be false is for an alphabetic character to be appended to `buf`.
		// If `buf` is empty, then no characters were appended, so `invalid` is true.
		if invalid {
			bail!("Invalid identifier '{buf}'!")
		};
		Ok(buf)
	}

	fn peek_char(&mut self) -> Result<char> {
		let next: char = self.next_sig_char_impl()?;
		self.cached_chars.push_back(next);
		Ok(next)
	}

	// next significant character
	fn next_sig_char(&mut self) -> Result<char> {
		if let Some(cached) = self.cached_chars.pop_back() {
			return Ok(cached);
		};
		self.next_sig_char_impl()
	}

	fn next_sig_char_impl(&mut self) -> Result<char> {
		let mut current: char = self.read_char_impl()?;
		while current.is_whitespace() {
			current = self.read_char_impl()?;
		}
		Ok(current)
	}

	fn read(&mut self, buf: &mut [u8]) -> Result<()> {
		self.src
			.read_exact(buf)
			.context("An error occurred trying to read bytes (Lexer)!")
	}

	fn read_char(&mut self) -> Result<char> {
		if let Some(cached) = self.cached_chars.pop_back() {
			return Ok(cached);
		};
		self.read_char_impl()
	}

	fn read_char_impl(&mut self) -> Result<char> {
		let [initial]: [u8; 1] = {
			let mut buf: [u8; 1] = [0; 1];
			self.read(&mut buf)?;
			buf
		};

		// SANITY(unusual): All ASCII chars are below 128.  They're always single-byte, so some simple checks can be done here.
		if initial < 128 {
			if DISALLOWED_ASCII_CHARS.contains(&initial) {
				// SANITY(unexpected):
				// Aside from malicious or invalid input, it isn't expected for any of these chars to appear.
				// A fair few of them are ancient ASCII sequences which most people will never run into in their entire lives.
				cold_path();
				bail!("Disallowed ASCII character appeared!  Character (in decimal): '{initial}'.");
			};
			// SANITY(fast-path): All ASCII chars are single-byte.
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
				bail!("Invalid UTF-8 byte (length of 0)!");
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

static DISALLOWED_ASCII_CHARS: [u8; 27] = [
	0,   // Null
	1,   // Start of Heading
	2,   // Start of Text
	3,   // End of Text
	4,   // End of Transmission
	5,   // Enquiry
	6,   // Acknowledge
	7,   // Bell
	8,   // Backspace
	12,  // Form Feed
	16,  // Data Link Escape
	17,  // Device Control One
	18,  // Device Control Two
	19,  // Device Control Three
	20,  // Device Control Four
	21,  // Negative Acknowledge
	22,  // Synchronous Idle
	23,  // End of Transmission Block
	24,  // Cancel
	25,  // End of medium
	26,  // Substitute,
	27,  // Escape
	28,  // File Separator
	29,  // Group Separator
	30,  // Record Separator
	31,  // Unit Separator
	127, // Delete
];

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
