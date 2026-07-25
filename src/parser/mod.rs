use anyhow::{Context as _, Result};

use derive_more::{Constructor, Display};

use crate::lexer::{Lexer, Source, Token};
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
	__Size,
};
use crate::types::{__Boolean, __Ptr, __Type, __Void};
use crate::{const_num_env, expect_token, unexpected_token, unreachable_ice, unwrap_identifier};

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

pub mod fn_attrs;
pub mod function;

use fn_attrs::{Attribute, Attributes};
use function::FunctionDeclaration;
use modifiers::{MOD_CONSTANT, MOD_EXTERNAL, MOD_PRIVATE, MOD_UNSAFE, Modifiers};

/// `topLevel : functionDecl ;`
#[must_use]
#[derive(Debug, PartialEq)]
#[repr(transparent)]
pub enum TopLevel {
	Function(FunctionDeclaration),
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

/// `expr : addExpr | subExpr | mulExpr | divExpr | unaryExpr | functionCallExpr | variableRefExpr | blockExpr ;`
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
	/// `variableRefExpr : IDENTIFIER ;`
	VariableRef(String),
	/// `blockExpr : ( CONSTANT? UNSAFE? )? OPEN_CURLY stmt* expr CLOSE_CURLY ;`
	Block(Modifiers, Vec<Stmt>, Box<Self>),
	IntegerLiteral(u128),
	// FIXME: Use f128 once stable (enough)
	FloatLiteral(f64),
}

#[derive(Debug, Display)]
pub enum Term {
	/// `+`
	Add,
	/// `-`
	Sub,
}

#[derive(Debug, Display)]
pub enum Factor {
	/// `*`
	Mul,
	/// `/`
	Div,
}

#[derive(Debug, Display)]
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

macro_rules! handle_erroneous {
	($self:expr, $lookahead:expr $(,)?) => {{
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
			let erroneous: Token = $self.lexer.read_token().expect("Cached.");
			unexpected_token!(erroneous);
		};
	}};
}

macro_rules! eat_unexpected_token {
	($self:expr) => {{
		unexpected_token!(
			$self
				.lexer
				.read_token()
				.expect("Unreachable panic, this read is cached."),
		)
	}};
}

#[derive_const(Constructor)]
pub struct Parser<'src> {
	lexer: Lexer<'src>,
}

const impl<'src> From<Lexer<'src>> for Parser<'src> {
	fn from(value: Lexer<'src>) -> Self {
		Self::new(value)
	}
}

const impl<'src> From<Source<'src>> for Parser<'src> {
	fn from(value: Source<'src>) -> Self {
		Self::new(Lexer::from(value))
	}
}

impl Parser<'_> {
	pub fn parse(&mut self) -> Result<TopLevel> {
		self.parse_top_level()
	}

	pub fn parse_top_level(&mut self) -> Result<TopLevel> {
		let mut modifiers: Modifiers = Modifiers::default();
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
			if self.lexer.try_eat_shebang()? {
				return self.parse_top_level();
			};
			let mut attributes: Attributes = Attributes::default();
			self.parse_function_attrs(&mut attributes)?;
			attributes
		} else {
			Attributes::default()
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
			_ => eat_unexpected_token!(self),
		};
		// Eat 'funct'.
		expect_token!(self.lexer.read_token()?, Token::Function);
		// Eat `identifier`.
		let identifier: String = unwrap_identifier!(self);
		let parameters: Vec<(String, __Type)> = self.parse_function_parameters()?;
		let return_type: __Type = if *self.lexer.peek_token()? == Token::Colon {
			// Eat ':'.
			let _ = self.lexer.read_token();
			self.parse_type()?
		} else {
			Box::new(__Void::instance())
		};
		match *self.lexer.peek_token()? {
			Token::Semicolon => {
				// Eat ';'.
				let _ = self.lexer.read_token();
			},
			Token::OpenCurlyBracket => {
				// TODO:
				// Attach this to function declaration?
				// What happens if a function is declared twice?
				// Implement a lookup for existing declarations?
				let _ = dbg!(self.parse_block_expr()?);
			},
			_ => eat_unexpected_token!(self),
		};
		let declaration: FunctionDeclaration =
			FunctionDeclaration::new(modifiers, attributes, identifier, parameters, return_type);
		Ok(TopLevel::Function(declaration))
	}

	pub fn parse_function_parameters(&mut self) -> Result<Vec<(String, __Type)>> {
		// Eat '('.
		expect_token!(self.lexer.read_token()?, Token::OpenBracket);

		// SANITY(fast-path): Immediate return if the next token is ')'.
		if *self.lexer.peek_token()? == Token::CloseBracket {
			// Eat ')'.
			let _ = self.lexer.read_token();
			return Ok(Vec::new());
		};

		let mut result: Vec<(String, __Type)> =
			Vec::with_capacity(const_num_env!("__PARAM_DECL_BUF_CAPACITY", 4));
		loop {
			let Token::Identifier(_) = *self.lexer.peek_token()? else {
				// Eat ')'.
				expect_token!(self.lexer.read_token()?, Token::CloseBracket);
				break;
			};
			let identifier: String = unwrap_identifier!(self);
			// Eat ':'.
			expect_token!(self.lexer.read_token()?, Token::Colon);
			let param_type: __Type = self.parse_type()?;
			result.push((identifier, param_type));
			if *self.lexer.peek_token()? == Token::Comma {
				// Eat ','.
				let _ = self.lexer.read_token();
			};
		}
		Ok(result)
	}

	pub fn parse_function_attrs(&mut self, output: &mut Attributes) -> Result<()> {
		// Eat '#'.
		expect_token!(self.lexer.read_token()?, Token::Hash);
		// Eat '('.
		expect_token!(self.lexer.read_token()?, Token::OpenBracket);
		loop {
			*output |= self.parse_function_attribute()?;
			match self.lexer.read_token()? {
				Token::Comma => {
					// Don't try to continue if the next token is ')'.
					if *self.lexer.peek_token()? == Token::CloseBracket {
						// Eat ')'.
						let _ = self.lexer.read_token();
						break;
					};
				},
				Token::CloseBracket => break,
				unexpected => unexpected_token!(unexpected),
			};
		}
		if *self.lexer.peek_token()? == Token::Hash {
			self.parse_function_attrs(output)?;
		};
		Ok(())
	}

	pub fn parse_function_attribute(&mut self) -> Result<Attribute> {
		let token: Token = self.lexer.read_token()?;
		match token {
			Token::Identifier(identifier) => match identifier.as_str() {
				"cold" => Ok(Attribute::Cold),
				"hot" => Ok(Attribute::Hot),
				"strictfp" => Ok(Attribute::Strictfp),
				"try_inline" => Ok(Attribute::TryInline),
				"force_inline" => Ok(Attribute::ForceInline),
				"method" => {
					// Eat '['.
					expect_token!(self.lexer.read_token()?, Token::OpenSquareBracket);
					let attached_type: String = unwrap_identifier!(self);
					// Eat ']'.
					expect_token!(self.lexer.read_token()?, Token::CloseSquareBracket);
					Ok(Attribute::Method(attached_type))
				},
				// SANITY(unusual): Parital move means we can't just pass `token`.
				_ => unexpected_token!(Token::Identifier(identifier)),
			},
			Token::Unsafe => {
				// Eat '('.
				expect_token!(self.lexer.read_token()?, Token::OpenBracket);
				let attribute: Attribute = match self.lexer.read_token()? {
					Token::Identifier(identifier) if identifier == "pure" =>
						Attribute::Purity(true),
					Token::Identifier(identifier) if identifier == "impure" =>
						Attribute::Purity(false),
					unexpected => unexpected_token!(unexpected),
				};
				// Eat ')'.
				expect_token!(self.lexer.read_token()?, Token::CloseBracket);
				Ok(attribute)
			},
			_ => unexpected_token!(token),
		}
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
					handle_erroneous!(self, lookahead);
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
					handle_erroneous!(self, lookahead);
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
		match op {
			Some(unary) => {
				// Eat `op`.
				let _ = self.lexer.read_token();
				Ok(Expr::Unary(
					unary,
					Box::new(self.parse_function_call_expr()?),
				))
			},
			None => self.parse_function_call_expr(),
		}
	}

	pub fn parse_function_call_expr(&mut self) -> Result<Expr> {
		let Token::Identifier(_): Token = *self.lexer.peek_token()? else {
			return self.parse_primary_expr();
		};
		// SANITY(unusual): This is cheaper than calling `<&String>::to_owned` to duplicate the reference from peeking.
		// SANITY(unreachable): The above check eliminates the possibility of the refute branch being reached.
		let identifier: String = unwrap_identifier!(@[unreachable = ICE] self);

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

		let mut parameters: Vec<Expr> =
			Vec::with_capacity(const_num_env!("__PARAM_CALL_BUF_CAPACITY", 4));
		loop {
			parameters.push(self.parse_primary_expr()?);
			match self.lexer.read_token()? {
				Token::CloseBracket => break,
				Token::Comma => (),
				unexpected => unexpected_token!(unexpected),
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
				let trimmed: &str = &real[..(real.len() - 1)];
				let value: f64 = trimmed
					.parse()
					.context("Couldn't parse floating-point literal.")?;
				// Eat `value`.
				let _ = self.lexer.read_token();
				Expr::FloatLiteral(value)
			},
			_ => eat_unexpected_token!(self),
		};
		Ok(expr)
	}

	pub fn parse_block_expr(&mut self) -> Result<Expr> {
		let mut modifiers: Modifiers = Modifiers::default();
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
			unexpected => unexpected_token!(unexpected),
		};
		let mut stmts: Vec<Stmt> = Vec::with_capacity(const_num_env!("__BLOCK_BUF_CAPACITY", 4));
		let result_expr: Expr = loop {
			if let Some(expr) = self.parse_block_body(&mut stmts)? {
				// Eat '}'.
				expect_token!(self.lexer.read_token()?, Token::CloseCurlyBracket);
				break expr;
			};
		};
		let expr: Expr = Expr::Block(modifiers, stmts, Box::new(result_expr));
		Ok(expr)
	}

	pub fn parse_block_body(&mut self, dest: &mut Vec<Stmt>) -> Result<Option<Expr>> {
		match *self.lexer.peek_token()? {
			Token::Val => dest.push(self.parse_variable_stmt()?),
			Token::Identifier(_) => {
				let identifier: String = unwrap_identifier!(self);
				match self.lexer.read_token()? {
					Token::Equals => {
						let expr: Expr = self.parse_expr()?;
						// Eat ';'.
						expect_token!(self.lexer.read_token()?, Token::Semicolon);
						dest.push(Stmt::Assignment(identifier, expr));
					},
					Token::CloseCurlyBracket => return Ok(Some(Expr::VariableRef(identifier))),
					Token::Semicolon => dest.push(Stmt::Unit(Expr::VariableRef(identifier))),
					unexpected => unexpected_token!(unexpected),
				};
			},
			// TODO(diagnostic): "Block expr must consist of at least one expr".
			Token::CloseCurlyBracket => unexpected_token!(Token::CloseCurlyBracket),
			_ => {
				let expr: Expr = self.parse_expr()?;
				if matches!(self.lexer.peek_token()?, Token::Semicolon) {
					// Eat ';'.
					let _ = self.lexer.read_token();
					dest.push(Stmt::Unit(expr));
				} else {
					return Ok(Some(expr));
				};
			},
		};
		Ok(None)
	}

	pub fn parse_stmt(&mut self) -> Result<Stmt> {
		let stmt: Stmt = match *self.lexer.peek_token()? {
			Token::Val => self.parse_variable_stmt()?,
			Token::Identifier(_) => self.parse_assignment_stmt()?,
			_ => self.parse_unit_stmt()?,
		};
		Ok(stmt)
	}

	pub fn parse_variable_stmt(&mut self) -> Result<Stmt> {
		// Eat 'val'.
		expect_token!(self.lexer.read_token()?, Token::Val);
		// Eat `identifier`.
		let identifier: String = unwrap_identifier!(self);
		// TODO: types on variables
		let assignment: Option<Expr> = match self.lexer.read_token()? {
			Token::Semicolon => None,
			Token::Equals => {
				let Stmt::Unit(expr): Stmt = self.parse_unit_stmt()? else {
					// SANITY(unreachable): The matched function always returns `Stmt::Unit`.
					unreachable_ice!(
						"`Parser::parse_unit_stmt` should have returned `Stmt::Unit`!",
						Parsing,
					);
				};
				Some(expr)
			},
			unexpected => unexpected_token!(unexpected),
		};
		Ok(Stmt::Variable(identifier, assignment))
	}

	pub fn parse_assignment_stmt(&mut self) -> Result<Stmt> {
		// Eat `identifier`.
		let identifier: String = unwrap_identifier!(self);
		// Eat '='.
		expect_token!(self.lexer.read_token()?, Token::Equals);
		let Stmt::Unit(expr): Stmt = self.parse_unit_stmt()? else {
			// SANITY(unreachable): The matched function always returns `Stmt::Unit`.
			unreachable_ice!(
				"`Parser::parse_unit_stmt` should have returned `Stmt::Unit`!",
				Parsing,
			);
		};
		Ok(Stmt::Assignment(identifier, expr))
	}

	pub fn parse_unit_stmt(&mut self) -> Result<Stmt> {
		let expr: Expr = self.parse_expr()?;
		// Eat ';'.
		expect_token!(self.lexer.read_token()?, Token::Semicolon);
		Ok(Stmt::Unit(expr))
	}

	// FIXME: (Typed) pointers & arrays / slices
	pub fn parse_type(&mut self) -> Result<__Type> {
		if let Token::Identifier(_) = *self.lexer.peek_token()? {
			let type_name: String = unwrap_identifier!(@[unreachable = ICE] self);
			return self.lookup_type(&type_name);
		};
		let _pointee_type_name: String = loop {
			match self.lexer.read_token()? {
				Token::Identifier(type_name) => break type_name,
				Token::Asterisk => match self.lexer.read_token()? {
					Token::Constant => (),
					unexpected => unexpected_token!(unexpected),
				},
				unexpected => unexpected_token!(unexpected),
			};
		};
		Ok(Box::new(__Ptr::instance()))
	}
}

impl Parser<'_> {
	pub fn lookup_type(&self, identifier: &str) -> Result<__Type> {
		if identifier.is_empty() || !identifier.is_ascii() {
			unreachable_ice!(
				"An identifier which was empty or contained non-ASCII characters was found!",
				Parsing,
			);
		};
		macro_rules! into {
			($ty:expr) => {
				Box::new($ty)
			};
		}
		// SANITY(ptr) + SAFETY: `identifier` is a non-empty ASCII string slice.
		let integer_is_unsigned: bool = unsafe { *identifier.as_ptr() } == b'u';
		Ok(match identifier {
			"void" => into!(__Void::instance()),
			"bool" => into!(__Boolean::instance()),
			"u8" | "i8" => into!(__8BitInteger::new(integer_is_unsigned)),
			"u16" | "i16" => into!(__16BitInteger::new(integer_is_unsigned)),
			"u32" | "i32" => into!(__32BitInteger::new(integer_is_unsigned)),
			"u64" | "i64" => into!(__64BitInteger::new(integer_is_unsigned)),
			"u128" | "i128" => into!(__128BitInteger::new(integer_is_unsigned)),
			"usize" | "isize" => into!(__Size::new(integer_is_unsigned)),
			"f16" => into!(__16BitFloatingPoint::instance()),
			"b16" => into!(__16BitBrainFloatingPoint::instance()),
			"f32" => into!(__32BitFloatingPoint::instance()),
			"f64" => into!(__64BitFloatingPoint::instance()),
			"f128" => into!(__128BitFloatingPoint::instance()),
			_ => todo!("Custom types (i.e. structs)"),
		})
	}
}
