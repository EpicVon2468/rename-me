use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hint::{cold_path, unreachable_unchecked};

use anyhow::{Context as _, Result, bail};

use crate::lexer::{Lexer, Src, Token};
use crate::types::floats::{
	__16BitBrainFloatingPoint,
	__16BitFloatingPoint,
	__32BitFloatingPoint,
	__64BitFloatingPoint,
	__128BitFloatingPoint,
};
use crate::types::integers::{
	__8BitInteger,
	__16BitInteger,
	__32BitInteger,
	__64BitInteger,
	__128BitInteger,
};
use crate::types::{__Boolean, __Type};

pub mod attrs {
	pub type Attributes = u8;

	macro_rules! attr_checker {
		($fn_name:ident, $attr:expr $(,)?) => {
			#[inline(always)]
			pub const fn $fn_name(attributes: Attributes) -> bool {
				attributes & $attr != 0
			}
		};
	}

	pub const ATTR_CONSTANT: Attributes = 0b0001;
	attr_checker!(is_constant, ATTR_CONSTANT);

	pub const ATTR_UNSAFE: Attributes = 0b0010;
	attr_checker!(is_unsafe, ATTR_UNSAFE);

	pub const ATTR_EXTERNAL: Attributes = 0b0100;
	attr_checker!(is_external, ATTR_EXTERNAL);

	pub const ATTR_PRIVATE: Attributes = 0b1000;
	attr_checker!(is_private, ATTR_PRIVATE);
}

use attrs::{ATTR_CONSTANT, ATTR_UNSAFE, Attributes};

pub struct FunctionDeclaration {
	pub attributes: Attributes,
	pub identifier: String,
	// TODO: identifier : type
	pub parameters: HashMap<String, String>,
	pub return_type: String,
}

/// `stmt : variableStmt | assignmentStmt | unitStmt ;`
#[derive(Debug)]
pub enum Stmt {
	/// `variableStmt : VAL ( assignmentStmt | IDENTIFIER ) ;`
	Variable(String, Option<Expr>),
	/// `assignmentStmt : IDENTIFIER EQUALS unitStmt ;`
	Assignment(String, Expr),
	/// `unitStmt : expr SEMICOLON ;`
	Unit(Expr),
}

#[derive(Debug)]
pub enum Expr {
	/// `addExpr : expr PLUS expr ;`
	Add(Box<Self>, Box<Self>),
	/// `subExpr : expr MINUS expr ;`
	Sub(Box<Self>, Box<Self>),
	/// `mulExpr : expr ASTERISK expr ;`
	Mul(Box<Self>, Box<Self>),
	/// `divExpr : expr SLASH expr ;`
	Div(Box<Self>, Box<Self>),
	/// `unaryExpr : ( MINUS | EXCLAMATION_MARK | ASTERISK | AMPERSAND ) expr ;`
	Unary(Unary, Box<Self>),
	/// `functionCallExpr : IDENTIFIER OPEN_BRACKET ( expr ( COMMA expr )* )? CLOSE_BRACKET ;`
	FunctionCall(String, Vec<Self>),
	VariableRef(String),
	/// `blockExpr : ( CONSTANT? UNSAFE? )? OPEN_CURLY stmt* expr CLOSE_CURLY ;`
	Block(Attributes, Vec<Stmt>, Box<Self>),
	IntegerLiteral(u128),
	// FIXME: Use f128 once RustRover supports it (LLVM has explicit support for it).
	FloatLiteral(f64),
}

pub enum Term {
	Add,
	Sub,
}

pub enum Factor {
	Mul,
	Div,
}

#[derive(Debug)]
pub enum Unary {
	/// `-`
	Minus,
	/// `!`
	Negate,
	/// `*`
	Ptr,
	/// `&`
	Ref,
}

pub struct Parser<'a> {
	lexer: Lexer<'a>,
}

macro_rules! expect_token {
	($actual:expr, $expected:expr $(,)?) => {{
		let token = $actual;
		let expected = $expected;
		if token != expected {
			bail!("Unexpected token reached, expected '{expected:?}', got '{token:?}'!");
		};
	}};
}

macro_rules! handle_erroneous {
	($lexer:expr, $lookahead:expr) => {{
		if matches!(
			$lookahead,
			Token::Identifier(_)
				| Token::Literal(_)
				| Token::Real(_)
				| Token::Function
				| Token::Unsafe
				| Token::External
				| Token::Constant
				| Token::Return
				| Token::Val | Token::OpenCurlyBracket
				| Token::OpenBracket
				| Token::OpenSquareBracket
				| Token::Colon
				| Token::ExclamationMark
		) {
			let erroneous: Token = $lexer.read_token().expect("Cached.");
			bail!(UnexpectedTokenError {
				unexpected: erroneous
			});
		};
	}};
}

impl Parser<'_> {
	#[must_use]
	pub fn type_by_name(&self, identifier: &str) -> Box<dyn __Type> {
		#[expect(clippy::panic)]
		if identifier.is_empty() || !identifier.is_ascii() {
			// SANITY(unexpected):
			// Given internal parser & lexer validation, this branch should be near-impossible to reach.
			cold_path();
			panic!("Identifiers must be non-empty and consist only of ASCII characters!");
		};
		macro_rules! bx {
			($ty:expr) => {
				Box::new($ty)
			};
		}
		// SANITY(ptr) + SAFETY: `identifier` is a non-empty ASCII string slice.
		let integer_is_unsigned: bool = unsafe { *identifier.as_ptr() } == b'u';
		match identifier {
			"bool" => bx!(__Boolean::instance()),
			"u8" | "i8" => bx!(__8BitInteger::new(integer_is_unsigned)),
			"u16" | "i16" => bx!(__16BitInteger::new(integer_is_unsigned)),
			"u32" | "i32" => bx!(__32BitInteger::new(integer_is_unsigned)),
			"u64" | "i64" => bx!(__64BitInteger::new(integer_is_unsigned)),
			"u128" | "i128" => bx!(__128BitInteger::new(integer_is_unsigned)),
			"f16" => bx!(__16BitFloatingPoint::instance()),
			"b16" => bx!(__16BitBrainFloatingPoint::instance()),
			"f32" => bx!(__32BitFloatingPoint::instance()),
			"f64" => bx!(__64BitFloatingPoint::instance()),
			"f128" => bx!(__128BitFloatingPoint::instance()),
			_ => todo!("Custom type lookup"),
		}
	}
}

impl<'a> Parser<'a> {
	#[must_use]
	pub const fn new(lexer: Lexer<'a>) -> Self {
		Self { lexer }
	}

	// Implementing the `From` trait is being weird.
	#[must_use]
	pub fn from(src: Src<'a>) -> Self {
		Self::new(Lexer::new(src))
	}

	pub fn parse(&mut self) -> Result<()> {
		let _ = dbg!(self.parse_expr()?);
		Ok(())
	}

	pub fn parse_expr(&mut self) -> Result<Expr> {
		self.parse_term_expr()
	}

	pub fn parse_term_expr(&mut self) -> Result<Expr> {
		let mut result: Expr = self.parse_factor_expr()?;
		loop {
			let lookahead: &Token = self.lexer.peek_token()?;
			let op: Term = match *lookahead {
				Token::Plus => Term::Add,
				Token::Minus => Term::Sub,
				_ => {
					handle_erroneous!(self.lexer, lookahead);
					break;
				},
			};
			// Eat `op`.
			let _ = self.lexer.read_token();
			let rhs: Expr = self.parse_factor_expr()?;
			result = match op {
				Term::Add => Expr::Add(Box::new(result), Box::new(rhs)),
				Term::Sub => Expr::Sub(Box::new(result), Box::new(rhs)),
			};
		}
		Ok(result)
	}

	pub fn parse_factor_expr(&mut self) -> Result<Expr> {
		let mut result: Expr = self.parse_unary_expr()?;
		loop {
			let lookahead: &Token = self.lexer.peek_token()?;
			let op: Factor = match *lookahead {
				Token::Asterisk => Factor::Mul,
				Token::Slash => Factor::Div,
				_ => {
					handle_erroneous!(self.lexer, lookahead);
					break;
				},
			};
			// Eat `op`.
			let _ = self.lexer.read_token();
			let rhs: Expr = self.parse_unary_expr()?;
			result = match op {
				Factor::Mul => Expr::Mul(Box::new(result), Box::new(rhs)),
				Factor::Div => Expr::Div(Box::new(result), Box::new(rhs)),
			};
		}
		Ok(result)
	}

	pub fn parse_unary_expr(&mut self) -> Result<Expr> {
		let op: Option<Unary> = match *self.lexer.peek_token()? {
			Token::Minus => Some(Unary::Minus),
			Token::ExclamationMark => Some(Unary::Negate),
			Token::Asterisk => Some(Unary::Ptr),
			Token::Ampersand => Some(Unary::Ref),
			_ => None,
		};
		let result: Expr = if let Some(unary) = op {
			// Eat `op`.
			let _ = self.lexer.read_token();
			Expr::Unary(unary, Box::new(self.parse_function_expr()?))
		} else {
			self.parse_function_expr()?
		};
		Ok(result)
	}

	/// Shorthand to prevent cloning the body of [`Token::Identifier`] on a peek match.
	///
	/// # Safety
	///
	/// This function is _only_ safe to call if the previously [`peeked`][`Lexer::peek_token`] token was [`Token::Identifier`].
	///
	/// Calling this in any other case is Undefined Behaviour.
	///
	/// Callers are responsible for upholding the contract as specified.
	pub unsafe fn take_identifier(&mut self) -> String {
		let Ok(Token::Identifier(identifier)): Result<Token> = self.lexer.read_token() else {
			// SANITY(unreachable):
			// `Lexer` caches the last peeked token and returns it on next read.
			// This means that a subsequent call to `Lexer::read_token()` after a call to `Lexer::peek_token()` will always return the same value.
			// No actual I/O operations occur during the subsequent call, thus errors are also impossible.
			// SAFETY:
			// Problem(s):
			// - `unreachable_unchecked()` is unsafe, and it is Undefined Behaviour for it to be reached.
			// Excuse(s):
			// - This statement cannot be reached.
			unsafe {
				unreachable_unchecked();
			};
		};
		identifier
	}

	pub fn parse_function_expr(&mut self) -> Result<Expr> {
		// FIXME: This conflicts with identifiers.  Can't peek ahead multiple times because of lexer limitations.
		let Token::Identifier(_): Token = *self.lexer.peek_token()? else {
			return self.parse_primary_expr();
		};
		// SANITY(unusual): This is cheaper than calling `<&String>::to_owned` to duplicate the reference from peeking.
		// SAFETY: The code above this line confirms that the next token will be `Token::Identifier`.
		let identifier: String = unsafe { self.take_identifier() };

		// Eat '('.
		expect_token!(self.lexer.read_token()?, Token::OpenBracket);

		// SANITY(fast-path): No need to allocate a proper `Vec` or loop if there are no parameters.
		if self.lexer.peek_token()? == &Token::CloseBracket {
			// Eat ')'.
			let _ = self.lexer.read_token();
			return Ok(Expr::FunctionCall(identifier, Vec::new()));
		};

		let mut parameters: Vec<Expr> = Vec::with_capacity(4);
		loop {
			parameters.push(self.parse_primary_expr()?);
			let token: Token = self.lexer.read_token()?;
			if token == Token::CloseBracket {
				break;
			} else if token != Token::Comma {
				bail!("Expected closing bracket or a comma, but instead found '{token:?}'!");
			};
		}
		Ok(Expr::FunctionCall(identifier, parameters))
	}

	pub fn parse_primary_expr(&mut self) -> Result<Expr> {
		let expr: Expr = match *self.lexer.peek_token()? {
			Token::Constant | Token::Unsafe | Token::OpenCurlyBracket => self.parse_block_expr()?,
			Token::OpenBracket => {
				// Eat '('.
				let _ = self.lexer.read_token();
				let result: Expr = self.parse_expr()?;
				// Eat ')'.
				expect_token!(self.lexer.read_token()?, Token::CloseBracket);
				result
			},
			// SANITY(unusual): This is cheaper than calling `<&String>::to_owned` to duplicate the reference from peeking.
			// SAFETY: This match arm ensures the next token will be `Token::Identifier`.
			Token::Identifier(_) => Expr::VariableRef(unsafe { self.take_identifier() }),
			Token::Literal(ref literal) => {
				let value: u128 = literal.parse().context("Couldn't parse integer literal.")?;
				// Eat `value`.
				let _ = self.lexer.read_token();
				Expr::IntegerLiteral(value)
			},
			Token::Real(ref real) => {
				// Trim 'f' suffix so f64 can parse it.
				let trimmed: &str = &real[0..(real.len() - 1)];
				let value: f64 = trimmed
					.parse()
					.context("Couldn't parse floating-point literal.")?;
				// Eat `value`.
				let _ = self.lexer.read_token();
				Expr::FloatLiteral(value)
			},
			_ => todo!("Is this unreachable?"),
		};
		Ok(expr)
	}

	pub fn parse_block_expr(&mut self) -> Result<Expr> {
		let mut attributes: Attributes = 0;
		match self.lexer.read_token()? {
			Token::Constant => {
				attributes |= ATTR_CONSTANT;
				if self.lexer.peek_token()? == &Token::Unsafe {
					// Eat 'unsafe'.
					let _ = self.lexer.read_token();
					attributes |= ATTR_UNSAFE;
				};
				// Eat '{'.
				expect_token!(self.lexer.read_token()?, Token::OpenCurlyBracket);
			},
			Token::Unsafe => {
				attributes |= ATTR_UNSAFE;
				// Eat '{'.
				expect_token!(self.lexer.read_token()?, Token::OpenCurlyBracket);
			},
			Token::OpenCurlyBracket => (),
			unexpected => bail!(UnexpectedTokenError { unexpected }),
		};
		let mut result: Vec<Stmt> = Vec::with_capacity(4);
		let expr: Expr = Expr::Block(attributes, todo!(), todo!());
		Ok(expr)
	}

	pub fn parse_stmt(&mut self) -> Result<Stmt> {
		let stmt: Stmt = match *self.lexer.peek_token()? {
			Token::Val => self.parse_variable_stmt()?,
			Token::Identifier(_) => self.parse_assignment_stmt()?,
			_ => self.parse_unit_stmt()?,
		};
		Ok(stmt)
	}

	fn peek_identifier_or_bail(&mut self) -> Result<()> {
		let Token::Identifier(_): Token = *self.lexer.peek_token()? else {
			bail!(UnexpectedTokenError {
				unexpected: self
					.lexer
					.read_token()
					.expect("Unreachable panic, this read is cached."),
			});
		};
		Ok(())
	}

	pub fn parse_variable_stmt(&mut self) -> Result<Stmt> {
		let Token::Val: Token = *self.lexer.peek_token()? else {
			bail!(UnexpectedTokenError {
				unexpected: self
					.lexer
					.read_token()
					.expect("Unreachable panic, this read is cached."),
			});
		};
		// Eat 'val'.
		let _ = self.lexer.read_token();
		self.peek_identifier_or_bail()?;
		let identifier: String;
		let assignment: Option<Expr> = if self.lexer.peek_token()? == &Token::Semicolon {
			// SANITY(unusual): This is cheaper than calling `<&String>::to_owned` to duplicate the reference from peeking.
			// SAFETY: The above check ensures the next token will be `Token::Identifier`.
			identifier = unsafe { self.take_identifier() };
			None
		} else {
			let Stmt::Assignment(ident, expr): Stmt = self.parse_assignment_stmt()? else {
				// SANITY(unreachable): The matched function always returns `Stmt::Assignment`.
				// SAFETY:
				// Problem(s):
				// - `unreachable_unchecked()` is unsafe, and it is Undefined Behaviour for it to be reached.
				// Excuse(s):
				// - This statement cannot be reached.
				unsafe {
					unreachable_unchecked();
				};
			};
			identifier = ident;
			Some(expr)
		};
		Ok(Stmt::Variable(identifier, assignment))
	}

	pub fn parse_assignment_stmt(&mut self) -> Result<Stmt> {
		self.peek_identifier_or_bail()?;
		// SANITY(unusual): This is cheaper than calling `<&String>::to_owned` to duplicate the reference from peeking.
		// SAFETY: The above check ensures the next token will be `Token::Identifier`.
		let identifier: String = unsafe { self.take_identifier() };
		// Eat '='.
		expect_token!(self.lexer.read_token()?, Token::Equals);
		let Stmt::Unit(expr): Stmt = self.parse_unit_stmt()? else {
			// SANITY(unreachable): The matched function always returns `Stmt::Unit`.
			// SAFETY:
			// Problem(s):
			// - `unreachable_unchecked()` is unsafe, and it is Undefined Behaviour for it to be reached.
			// Excuse(s):
			// - This statement cannot be reached.
			unsafe {
				unreachable_unchecked();
			};
		};
		Ok(Stmt::Assignment(identifier, expr))
	}

	pub fn parse_unit_stmt(&mut self) -> Result<Stmt> {
		let expr: Expr = self.parse_expr()?;
		// Eat ';'.
		expect_token!(self.lexer.read_token()?, Token::Semicolon);
		Ok(Stmt::Unit(expr))
	}
}

#[derive(Debug)]
pub struct UnexpectedTokenError {
	// TODO: `expected: Option<Token>,`
	unexpected: Token,
}

impl Display for UnexpectedTokenError {
	fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			fmt,
			"Unexpected token reached!  Actual token was {:?}!",
			self.unexpected,
		)
	}
}

impl Error for UnexpectedTokenError {}
