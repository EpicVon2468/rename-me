use std::collections::HashMap;

use anyhow::{Context as _, Result};

use crate::lexer::{Lexer, Src, Token};

pub mod attrs {
	pub type Attributes = u8;

	macro_rules! attr_checker {
		($fn_name:ident, $attr:expr $(,)?) => {
			pub const fn $fn_name(attributes: Attributes) -> bool {
				attributes & $attr != 0
			}
		};
	}

	pub const ATTR_ALL: Attributes = 0b1111;
	pub const ATTR_NONE: Attributes = 0b0000;

	pub const ATTR_CONSTANT: Attributes = 0b0001;
	attr_checker!(is_constant, ATTR_CONSTANT);

	pub const ATTR_UNSAFE: Attributes = 0b0010;
	attr_checker!(is_unsafe, ATTR_UNSAFE);

	pub const ATTR_EXTERNAL: Attributes = 0b0100;
	attr_checker!(is_external, ATTR_EXTERNAL);
}

use attrs::Attributes;

pub struct FunctionDeclaration {
	pub attributes: Attributes,
	pub identifier: String,
	// TODO: identifier : type
	pub parameters: HashMap<String, String>,
	pub return_type: String,
}

/// `stmt : variableStmt | assignmentStmt | unitStmt ;`
pub enum Stmt {
	// TODO: Option<Expr> for uninitialised variables?
	/// `variableStmt : VAL assignmentStmt ;`
	Variable(String, Expr),
	/// `assignmentStmt : IDENTIFIER EQUALS unitStmt ;`
	Assignment(String, Expr),
	/// `unitStmt : expr SEMICOLON ;`
	Unit(Expr),
}

/// `expr : termExpr ;`
#[repr(transparent)]
pub struct Expr(TermExpr);

/// `termExpr : factorExpr ( ( PLUS | MINUS ) factorExpr )* ;`
pub struct TermExpr {
	pub lhs: FactorExpr,
	pub next: Option<(Term, Box<Self>)>,
}

pub enum Term {
	Add,
	Sub,
}

/// `factorExpr : unaryExpr ( ( SLASH | ASTERISK ) unaryExpr )* ;`
pub struct FactorExpr {
	pub lhs: UnaryExpr,
	pub next: Option<(Factor, Box<Self>)>,
}

pub enum Factor {
	Div,
	Mul,
}

/// `unaryExpr : ( MINUS | ASTERISK | AMPERSAND )? functionExpr ;`
pub struct UnaryExpr {
	pub prefix: Option<Unary>,
	pub next: FunctionExpr,
}

pub enum Unary {
	/// `-`
	Minus,
	/// `*`
	Ptr,
	/// `&`
	Ref,
}

/// `functionExpr : ( IDENTIFIER OPEN_BRACKET ( primaryExpr ( COMMA primaryExpr )* )? CLOSE_BRACKET ) | primaryExpr ;`
pub struct FunctionExpr {
	pub identifier: Option<String>,
	pub expr: Vec<PrimaryExpr>,
}

/// `primaryExpr : IDENTIFIER | value | ( OPEN_BRACKET expr CLOSE_BRACKET ) | blockExpr ;`
pub enum PrimaryExpr {
	VariableRef(String),
	// TODO implement
	Value,
	Expr(Expr),
	Block(BlockExpr),
}

/// `blockExpr : ( CONSTANT? UNSAFE? )? OPEN_BRACE stmt* expr CLOSE_BRACE ;`
pub struct BlockExpr {
	attributes: Attributes,
	body: Vec<Stmt>,
	final_value: Expr,
}

pub struct Parser<'a> {
	lexer: Lexer<'a>,
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
		let result: Result<Expr> = self.parse_expr();
		match result {
			Err(error) => Err(error),
			Ok(_) => Ok(()),
		}
	}

	pub fn parse_expr(&mut self) -> Result<Expr> {
		self.parse_term_expr().map(Expr)
	}

	pub fn parse_term_expr(&mut self) -> Result<TermExpr> {
		let lhs: FactorExpr = self.parse_factor_expr()?;
		let mut result: TermExpr = TermExpr { lhs, next: None };
		self.recurse_term_expr(&mut result)?;
		Ok(result)
	}

	fn recurse_term_expr(&mut self, result: &mut TermExpr) -> Result<()> {
		let lookahead: &Token = self.lexer.peek_token()?;
		let op: Term = match *lookahead {
			Token::Plus => Term::Add,
			Token::Minus => Term::Sub,
			_ => return Ok(()),
		};
		// Eat `op`.
		let _ = self.lexer.read_token();
		let lhs: FactorExpr = self
			.parse_factor_expr()
			.context("Expected factorExpr to follow dangling operator in termExpr!")?;
		let mut next: TermExpr = TermExpr { lhs, next: None };
		self.recurse_term_expr(&mut next)?;
		result.next = Some((op, Box::new(next)));
		Ok(())
	}

	pub fn parse_factor_expr(&mut self) -> Result<FactorExpr> {
		todo!()
	}
}
