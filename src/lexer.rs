use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hint::{cold_path, unreachable_unchecked};
use std::io::{Error as IOError, Read};
use std::str::Utf8Error;

use anyhow::{Result, anyhow, bail};

use crate::const_num_env;

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
	/// `PRIVATE : 'p' 'r' 'i' 'v' 'a' 't' 'e' ;`
	Private,
	/// `RETURN : 'r' 'e' 't' 'u' 'r' 'n' ;`
	Return,
	/// `VAL : 'v' 'a' 'l' ;`
	Val,
	/// [`llvm.cold`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=cold,-This)
	///
	/// `COLD : 'l' 'l' 'v' 'm' '-' 'c' 'o' 'l' 'd' ;`
	Cold,
	/// [`llvm.hot`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=hot,-This)
	///
	/// `HOT : 'l' 'l' 'v' 'm' '-' 'h' 'o' 't' ;`
	Hot,
	/// [`llvm.strictfp`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=strictfp)
	///
	/// `STRICT_FP : 's' 't' 'r' 'i' 'c' 't' '-' 'f' 'p' ;`
	StrictFloatingPoint,
	/// [`llvm.inlinehint`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=inlinehint)
	///
	/// `TRY_INLINE : 't' 'r' 'y' '-' 'i' 'n' 'l' 'i' 'n' 'e' ;`
	TryInline,
	/// [`llvm.alwaysinline`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=alwaysinline,-This)
	///
	/// `FORCE_INLINE : 'f' 'o' 'r' 'c' 'e' '-' 'i' 'n' 'l' 'i' 'n' 'e' ;`
	ForceInline,
	/// `OPEN_CURLY : '{' ;`
	OpenCurlyBracket,
	/// `CLOSE_CURLY : '}' ;`
	CloseCurlyBracket,
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
	/// `HASH : '#' ;`
	Hash,
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
	input.is_ascii_alphanumeric() || input == '_' || input == '-'
}

pub type Source<'src> = &'src mut dyn Read;

pub struct Lexer<'src> {
	source: Source<'src>,
	cached: Option<Token>,
	cached_chars: VecDeque<char>,
}

const impl<'src> From<Source<'src>> for Lexer<'src> {
	fn from(value: Source<'src>) -> Self {
		Self::new(value)
	}
}

impl<'src> Lexer<'src> {
	pub const fn new(source: Source<'src>) -> Self {
		Self {
			source,
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
					"private" => Token::Private,
					"return" => Token::Return,
					"val" => Token::Val,
					"llvm-cold" => Token::Cold,
					"llvm-hot" => Token::Hot,
					"strict-fp" => Token::StrictFloatingPoint,
					"try-inline" => Token::TryInline,
					"force-inline" => Token::ForceInline,
					identifier => {
						if identifier.contains('-') {
							bail!("Illegal identifier '{identifier}'!  Contained a '-'!");
						};
						Token::Identifier(identifier.to_owned())
					},
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
			'{' => Token::OpenCurlyBracket,
			'}' => Token::CloseCurlyBracket,
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
			'#' => Token::Hash,
			other => bail!("Unrecognised character input '{other}'!"),
		})
	}

	fn read_num_chars(&mut self) -> Result<String> {
		let mut buf: String = String::with_capacity(const_num_env!("__NUM_BUF_CAPACITY", 16));
		let (mut invalid, mut seen_period): (bool, bool) = (true, false);
		let mut current: char = self.read_char()?;
		if !current.is_ascii_digit() {
			bail!("Invalid number start character '{current}'!");
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
				bail!("Unexpected character(s) found after real '{buf}'!  First was '{next}'!");
			};
			if is_next_significant {
				self.cached_chars.push_back(next);
			};
		} else if !current.is_whitespace() {
			// Push back significant chars because of accidental greedy consumption.
			self.cached_chars.push_back(current);
		};
		Ok(buf)
	}

	fn read_ident_chars(&mut self) -> Result<String> {
		let mut buf: String = String::with_capacity(const_num_env!("__IDENT_BUF_CAPACITY", 16));
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
		// Push back significant chars because of accidental greedy consumption.
		if !current.is_whitespace() {
			self.cached_chars.push_back(current);
		};
		// SANITY(unchecked):
		// No check is performed to determine whether `buf` is empty.
		// The `invalid` variable is true by default, the only way for it to be false is for an alphabetic character to be appended to `buf`.
		// If `buf` is empty, then no characters were appended, so `invalid` is true.
		if invalid {
			bail!("Invalid identifier '{buf}'!");
		};
		Ok(buf)
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
		self.source
			.read_exact(buf)
			.map_err(|error: IOError| anyhow!(ReadError::ForwardedIOError(error)))
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
				bail!(ReadError::DisallowedASCIIChar(initial));
			};
			#[expect(
				clippy::as_conversions,
				reason = "False positive.  This cast is directly equivalent to using `char::from(_)`."
			)]
			// SANITY(fast-path): All ASCII chars are single-byte.
			return Ok(initial as char);
		};

		if initial < 192 {
			bail!(ReadError::ZeroLengthByte(initial));
		};

		#[expect(
			clippy::as_conversions,
			reason = "False positive.  These casts are directly equivalent to `usize::from(_)`."
		)]
		let len: usize = {
			// SANITY(unusual):
			// 192 is subtracted because the value is always >= 192 at this point.
			// This means space can be saved by stripping the first 192 entries.
			// SAFETY:
			// Problem(s):
			// - `u8::unchecked_sub()` can underflow or overflow.
			// Excuse(s):
			// - The amount being subtracted is a trusted constant value (192).
			// - The value of `initial` is always >= 192 at this point, meaning subtracting 192 is always safe.
			let index: usize = unsafe { initial.unchecked_sub(192) } as usize;

			// Determine UTF-8 byte length for a multibyte char.
			// SANITY(unusual): Cheaper to store 128 `u8`s and convert at use-site than to store 128 `usize`s.
			let len: usize = UTF8_CHAR_WIDTH[index] as usize;
			if len == 0 {
				bail!(ReadError::ZeroLengthByte(initial));
			};
			len
		};

		let mut buf: Vec<u8> = vec![0; len];
		buf[0] = initial;
		self.read(&mut buf[1..])?;

		// TODO: Could use `buf.utf8_chunks().next()`...
		// SANITY(unusual): There doesn't appear to be any other way to make a `char` from a `u8` slice than this.
		let string: &str = match str::from_utf8(&buf) {
			Ok(result) => result,
			Err(error) => bail!(ReadError::InvalidByteSequence(Some(error), buf)),
		};
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

#[derive(Debug)]
pub enum ReadError {
	ForwardedIOError(IOError),
	DisallowedASCIIChar(u8),
	ZeroLengthByte(u8),
	InvalidByteSequence(Option<Utf8Error>, Vec<u8>),
}

impl Display for ReadError {
	fn fmt(&self, destination: &mut Formatter<'_>) -> std::fmt::Result {
		match *self {
			Self::ForwardedIOError(_) => write!(
				destination,
				"An I/O error occurred whilst trying to read bytes from source!",
			),
			Self::DisallowedASCIIChar(ref char) => write!(
				destination,
				"Disallowed ASCII character appeared!  Character (in decimal) was: '{char}'!",
			),
			Self::ZeroLengthByte(ref byte) => write!(
				destination,
				"Invalid UTF-8 byte with length of 0 appeared!  Byte (in decimal) was: '{byte}'!",
			),
			Self::InvalidByteSequence(_, ref sequence) => write!(
				destination,
				"Invalid UTF-8 byte sequence!  Sequence (in decimal(s)) was '{sequence:?}'!",
			),
		}
	}
}

impl Error for ReadError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match *self {
			Self::ForwardedIOError(ref source) => Some(source),
			Self::InvalidByteSequence(ref source, _) => source
				.as_ref()
				.map::<&(dyn Error + 'static), _>(|error: &Utf8Error| error),
			_ => None,
		}
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
// Copied from `core`'s str internals, w/o the first 192 entries.
// Cheaper to store 192 `u8`s and convert at use-site than to store 192 `usize`s.
// The first 128 entries (which were all 1) were stripped out, as the only use of this value is non-ASCII.
// The next 64 entries (which were all 0) were also stripped out.
// https://tools.ietf.org/html/rfc3629
static UTF8_CHAR_WIDTH: [u8; 64] = [
	// 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
	0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
	2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
	3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
	4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
];
