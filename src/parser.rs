use std::collections::HashMap;
use std::hint::unreachable_unchecked;
use std::str::FromStr as _;

use anyhow::{Context as _, Result, bail};

use crate::lexer::{Lexer, Src, Token};

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
	// TODO: Option<Expr> for uninitialised variables?
	/// `variableStmt : VAL assignmentStmt ;`
	Variable(String, Expr),
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
	/// `blockExpr : ( CONSTANT? UNSAFE? )? OPEN_BRACE stmt* expr CLOSE_BRACE ;`
	Block(Attributes, Vec<Stmt>, Box<Self>),
	IntegerLiteral(i128),
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
	// FIXME: This will cause problems since it follows right after `factorExpr`.
	//// `*`
	// Ptr,
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
				| Token::Val | Token::OpenBrace
				| Token::OpenBracket
				| Token::OpenSquareBracket
				| Token::Colon
				| Token::ExclamationMark
		) {
			let erroneous: Token = $lexer.read_token().expect("Cached.");
			bail!("Ran into erroneous token whilst parsing termExpr.  Token was: {erroneous:?}!");
		};
	}};
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
				Term::Add => Expr::Add(result.into(), rhs.into()),
				Term::Sub => Expr::Sub(result.into(), rhs.into()),
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
				Factor::Mul => Expr::Mul(result.into(), rhs.into()),
				Factor::Div => Expr::Div(result.into(), rhs.into()),
			};
		}
		Ok(result)
	}

	pub fn parse_unary_expr(&mut self) -> Result<Expr> {
		let op: Option<Unary> = match *self.lexer.peek_token()? {
			Token::Minus => Unary::Minus.into(),
			Token::ExclamationMark => Unary::Negate.into(),
			Token::Ampersand => Unary::Ref.into(),
			_ => None,
		};
		let result: Expr = if let Some(unary) = op {
			// Eat `op`.
			let _ = self.lexer.read_token();
			Expr::Unary(unary, self.parse_function_expr()?.into())
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
		// SANITY(unexpected): The next token should always be '('.
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
				bail!("Expected closing bracket or a comma, but instead found '{token:?}'!")
			};
		}
		Ok(Expr::FunctionCall(identifier, parameters))
	}

	pub fn parse_primary_expr(&mut self) -> Result<Expr> {
		let expr: Expr = match *self.lexer.peek_token()? {
			// SAFETY: This match arm ensures the next token is valid for a `blockExpr`.
			Token::Constant | Token::Unsafe | Token::OpenBrace => unsafe {
				self.parse_block_expr()?
			},
			Token::OpenBracket => {
				// Eat '('.
				let _ = self.lexer.read_token();
				let result: Expr = self.parse_expr()?;
				// SANITY(unexpected): The next token should always be ')'.
				expect_token!(self.lexer.read_token()?, Token::CloseBracket);
				result
			},
			// SANITY(unusual): This is cheaper than calling `<&String>::to_owned` to duplicate the reference from peeking.
			// SAFETY: This match arm ensures the next token will be `Token::Identifier`.
			Token::Identifier(_) => Expr::VariableRef(unsafe { self.take_identifier() }),
			Token::Literal(ref literal) => {
				let value: i128 =
					i128::from_str(literal).context("Couldn't parse integer literal.")?;
				// Eat `value`.
				let _ = self.lexer.read_token();
				Expr::IntegerLiteral(value)
			},
			Token::Real(ref real) => {
				// Trim 'f' suffix so f64 can parse it.
				let trimmed: &str = &real[0..(real.len() - 1)];
				let value: f64 =
					f64::from_str(trimmed).context("Couldn't parse floating-point literal.")?;
				// Eat `value`.
				let _ = self.lexer.read_token();
				Expr::FloatLiteral(value)
			},
			_ => todo!("Is this unreachable?"),
		};
		Ok(expr)
	}

	/// # Safety
	///
	/// Unlike the other parsing [`Expr`] parsing methods defined in [`Self`], this method _does not_ hand off parsing down to any other kind of expression on failure to match the correct initial token(s).
	///
	/// Callers must, _at minimum_ ensure that the next token is valid for a `blockExpr`.
	pub unsafe fn parse_block_expr(&mut self) -> Result<Expr> {
		let mut attributes: Attributes = 0;
		match self.lexer.read_token()? {
			Token::Constant => {
				attributes |= ATTR_CONSTANT;
				if self.lexer.peek_token()? == &Token::Unsafe {
					// SANITY(unchecked):
					// `Lexer` caches the last peeked token and returns it on next read.
					// This means that a subsequent call to `Lexer::read_token()` after a call to `Lexer::peek_token()` will always return the same value.
					// Therefore, it is always safe to assume this is the correct token and leave the return value unchecked.
					// This includes leaving the `Result` untouched, as no actual I/O operation occurs.
					let _ = self.lexer.read_token();
					attributes |= ATTR_UNSAFE;
				};
				// SANITY(unexpected): The next token should always be '{'.
				expect_token!(self.lexer.read_token()?, Token::OpenBrace);
			},
			Token::Unsafe => {
				attributes |= ATTR_UNSAFE;
				// SANITY(unexpected): The next token should always be '{'.
				expect_token!(self.lexer.read_token()?, Token::OpenBrace);
			},
			Token::OpenBrace => (),
			token => bail!("Unexpected token reached!  Token was '{token:?}'!"),
		};
		let expr: Expr = Expr::Block(attributes, todo!(), todo!());
		Ok(expr)
	}
}
