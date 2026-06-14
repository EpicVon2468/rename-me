use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
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
use crate::types::{__Boolean, __Type, __Void};

macro_rules! bitflag {
	($flag_ty:ty, $flag:expr, $fn_name:ident $(,)?) => {
		#[inline(always)]
		pub const fn $fn_name(input: $flag_ty) -> bool {
			input & $flag != 0
		}
	};
}

pub mod modifiers {
	use std::fmt::{Formatter, Result};

	pub type Modifiers = u8;
	pub fn fmt_modifiers(val: u8, fmt: &mut Formatter<'_>) -> Result {
		fmt.debug_struct("Modifiers")
			.field("is_constant", &is_constant(val))
			.field("is_unsafe", &is_unsafe(val))
			.field("is_external", &is_external(val))
			.field("is_private", &is_private(val))
			.finish()
	}

	pub const MOD_CONSTANT: Modifiers = 0b0001;
	bitflag!(Modifiers, MOD_CONSTANT, is_constant);

	pub const MOD_UNSAFE: Modifiers = 0b0010;
	bitflag!(Modifiers, MOD_UNSAFE, is_unsafe);

	pub const MOD_EXTERNAL: Modifiers = 0b0100;
	bitflag!(Modifiers, MOD_EXTERNAL, is_external);

	pub const MOD_PRIVATE: Modifiers = 0b1000;
	bitflag!(Modifiers, MOD_PRIVATE, is_private);
}

pub mod fn_attrs {
	use std::fmt::Formatter;

	use crate::lexer::Token;

	pub type Attributes = u8;
	pub fn fmt_attributes(val: u8, fmt: &mut Formatter<'_>) -> std::fmt::Result {
		fmt.debug_struct("Attributes")
			.field("is_cold", &is_cold(val))
			.field("is_hot", &is_hot(val))
			.field("is_strictfp", &is_strictfp(val))
			.field("is_try_inline", &is_try_inline(val))
			.field("is_force_inline", &is_force_inline(val))
			.finish()
	}

	#[must_use]
	pub const fn from_token(token: &Token) -> Option<Attributes> {
		match *token {
			Token::Cold => Some(ATTR_COLD),
			Token::Hot => Some(ATTR_HOT),
			Token::StrictFloatingPoint => Some(ATTR_STRICTFP),
			Token::TryInline => Some(ATTR_TRY_INLINE),
			Token::ForceInline => Some(ATTR_FORCE_INLINE),
			_ => None,
		}
	}

	pub const ATTR_COLD: Attributes = 0b0000_0001;
	bitflag!(Attributes, ATTR_COLD, is_cold);

	pub const ATTR_HOT: Attributes = 0b0000_0010;
	bitflag!(Attributes, ATTR_HOT, is_hot);

	pub const ATTR_STRICTFP: Attributes = 0b0000_0100;
	bitflag!(Attributes, ATTR_STRICTFP, is_strictfp);

	pub const ATTR_TRY_INLINE: Attributes = 0b0000_1000;
	bitflag!(Attributes, ATTR_TRY_INLINE, is_try_inline);

	pub const ATTR_FORCE_INLINE: Attributes = 0b0001_0000;
	bitflag!(Attributes, ATTR_FORCE_INLINE, is_force_inline);
}

use fn_attrs::{Attributes, fmt_attributes};
use modifiers::{MOD_CONSTANT, MOD_EXTERNAL, MOD_PRIVATE, MOD_UNSAFE, Modifiers, fmt_modifiers};

#[must_use]
#[derive(Debug)]
pub enum TopLevel {
	Function(FunctionDeclaration),
}

#[must_use]
pub struct FunctionDeclaration {
	pub modifiers: Modifiers,
	pub attributes: Attributes,
	pub identifier: String,
	// HashMap is not ordered
	pub parameters: Vec<(String, Box<dyn __Type>)>,
	pub return_type: Box<dyn __Type>,
}

impl Debug for FunctionDeclaration {
	fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
		fmt.debug_struct("FunctionDeclaration")
			.field_with("modifiers", |fmt: &mut Formatter<'_>| {
				fmt_modifiers(self.modifiers, fmt)
			})
			.field_with("attributes", |fmt: &mut Formatter<'_>| {
				fmt_attributes(self.attributes, fmt)
			})
			.field("identifier", &self.identifier)
			.field("parameters", &self.parameters)
			.field("return_type", &self.return_type)
			.finish()
	}
}

/// `stmt : variableStmt | assignmentStmt | unitStmt ;`
#[must_use]
#[derive(Debug)]
pub enum Stmt {
	/// `variableStmt : VAL ( assignmentStmt | IDENTIFIER ) ;`
	Variable(String, Option<Expr>),
	/// `assignmentStmt : IDENTIFIER EQUALS unitStmt ;`
	Assignment(String, Expr),
	/// `unitStmt : expr SEMICOLON ;`
	Unit(Expr),
}

#[must_use]
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
	Block(Modifiers, Vec<Stmt>, Box<Self>),
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
			"void" => bx!(__Void::instance()),
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

	pub fn parse(&mut self) -> Result<TopLevel> {
		self.parse_top_level()
	}

	pub fn parse_top_level(&mut self) -> Result<TopLevel> {
		let mut modifiers: Modifiers = 0;
		macro_rules! try_eat_extern {
			() => {
				if *self.lexer.peek_token()? == Token::External {
					// Eat 'extern'.
					let _ = self.lexer.read_token();
					modifiers |= MOD_EXTERNAL;
				};
			};
		}
		macro_rules! try_eat_unsafe_extern {
			() => {
				if *self.lexer.peek_token()? == Token::Unsafe {
					// Eat 'unsafe'.
					let _ = self.lexer.read_token();
					modifiers |= MOD_UNSAFE;
					try_eat_extern!();
				};
			};
		}
		let attributes: Attributes = if *self.lexer.peek_token()? == Token::Hash {
			self.parse_function_attrs()?
		} else {
			0
		};
		match *self.lexer.peek_token()? {
			// Continue down.
			Token::Function => (),
			Token::Private => {
				// Eat 'private'.
				let _ = self.lexer.read_token();
				modifiers |= MOD_PRIVATE;
				if *self.lexer.peek_token()? == Token::Constant {
					// Eat 'const'.
					let _ = self.lexer.read_token();
					modifiers |= MOD_CONSTANT;
				};
				try_eat_unsafe_extern!();
			},
			Token::Constant => {
				// Eat 'const'.
				let _ = self.lexer.read_token();
				modifiers |= MOD_CONSTANT;
				try_eat_unsafe_extern!();
			},
			Token::Unsafe => {
				// Eat 'unsafe'.
				let _ = self.lexer.read_token();
				modifiers |= MOD_UNSAFE;
				try_eat_extern!();
			},
			_ => bail!(UnexpectedTokenError {
				unexpected: self
					.lexer
					.read_token()
					.expect("Unreachable panic, this read is cached."),
			}),
		};
		// Eat 'funct'.
		expect_token!(self.lexer.read_token()?, Token::Function);
		let token: Token = self.lexer.read_token()?;
		let Token::Identifier(mut identifier): Token = token else {
			bail!(UnexpectedTokenError { unexpected: token });
		};
		identifier.push('\0');
		let parameters: Vec<(String, Box<dyn __Type>)> = self.parse_function_parameters()?;
		let return_type: Box<dyn __Type> = if *self.lexer.peek_token()? == Token::Colon {
			// Eat ':'.
			let _ = self.lexer.read_token();
			let token: Token = self.lexer.read_token()?;
			let Token::Identifier(return_type): Token = token else {
				bail!(UnexpectedTokenError { unexpected: token })
			};
			self.type_by_name(&return_type)
		} else {
			Box::new(__Void::instance())
		};
		let declaration: FunctionDeclaration = FunctionDeclaration {
			modifiers,
			attributes,
			identifier,
			parameters,
			return_type,
		};
		Ok(TopLevel::Function(declaration))
	}

	pub fn parse_function_parameters(&mut self) -> Result<Vec<(String, Box<dyn __Type>)>> {
		// Eat '('.
		expect_token!(self.lexer.read_token()?, Token::OpenBracket);

		// SANITY(fast-path): Immediate return if the next token is ')'.
		if *self.lexer.peek_token()? == Token::CloseBracket {
			return Ok(Vec::new());
		};
		todo!()
	}

	pub fn parse_function_attrs(&mut self) -> Result<Attributes> {
		// Eat '#'.
		expect_token!(self.lexer.read_token()?, Token::Hash);
		// Eat '('.
		expect_token!(self.lexer.read_token()?, Token::OpenBracket);

		let mut output: Attributes = 0;
		loop {
			let token: Token = self.lexer.read_token()?;
			if let Some(attr) = fn_attrs::from_token(&token) {
				output |= attr;
				match self.lexer.read_token()? {
					Token::Comma => {
						// Don't try to continue if the next token is ')'.
						if *self.lexer.peek_token()? == Token::CloseBracket {
							// Eat ')'.
							let _ = self.lexer.read_token();
							break;
						};
						continue;
					},
					Token::CloseBracket => break,
					unexpected => bail!(UnexpectedTokenError { unexpected }),
				};
			};
			bail!(UnexpectedTokenError { unexpected: token });
		}
		Ok(output)
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
			Expr::Unary(unary, Box::new(self.parse_function_call_expr()?))
		} else {
			self.parse_function_call_expr()?
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

	pub fn parse_function_call_expr(&mut self) -> Result<Expr> {
		let Token::Identifier(_): Token = *self.lexer.peek_token()? else {
			return self.parse_primary_expr();
		};
		// SANITY(unusual): This is cheaper than calling `<&String>::to_owned` to duplicate the reference from peeking.
		// SAFETY: The code above this line confirms that the next token will be `Token::Identifier`.
		let identifier: String = unsafe { self.take_identifier() };

		// SANITY(unusual): Hack to get variable references because of limitations with lexer peeking.
		if *self.lexer.peek_token()? != Token::OpenBracket {
			return Ok(Expr::VariableRef(identifier));
		};

		// Eat '('.
		let _ = self.lexer.read_token();

		// SANITY(fast-path): No need to allocate a proper `Vec` or loop if there are no parameters.
		if *self.lexer.peek_token()? == Token::CloseBracket {
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
			// // SANITY(unusual): This is cheaper than calling `<&String>::to_owned` to duplicate the reference from peeking.
			// // SAFETY: This match arm ensures the next token will be `Token::Identifier`.
			// Token::Identifier(_) => Expr::VariableRef(unsafe { self.take_identifier() }),
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
		let mut modifiers: Modifiers = 0;
		match self.lexer.read_token()? {
			Token::Constant => {
				modifiers |= MOD_CONSTANT;
				if *self.lexer.peek_token()? == Token::Unsafe {
					// Eat 'unsafe'.
					let _ = self.lexer.read_token();
					modifiers |= MOD_UNSAFE;
				};
				// Eat '{'.
				expect_token!(self.lexer.read_token()?, Token::OpenCurlyBracket);
			},
			Token::Unsafe => {
				modifiers |= MOD_UNSAFE;
				// Eat '{'.
				expect_token!(self.lexer.read_token()?, Token::OpenCurlyBracket);
			},
			Token::OpenCurlyBracket => (),
			unexpected => bail!(UnexpectedTokenError { unexpected }),
		};
		let mut result: Vec<Stmt> = Vec::with_capacity(4);
		let expr: Expr = Expr::Block(modifiers, todo!(), todo!());
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
