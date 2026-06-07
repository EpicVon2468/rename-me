use anyhow::Result;

use crate::lexer::{Lexer, Src, Token};

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
	pub next: Option<(Term, FactorExpr)>,
}

pub enum Term {
	Add,
	Sub,
}

/// `factorExpr : unaryExpr ( ( SLASH | ASTERISK ) unaryExpr )* ;`
pub struct FactorExpr {
	pub lhs: UnaryExpr,
	pub next: Option<(Factor, UnaryExpr)>,
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
	is_unsafe: bool,
	is_const: bool,
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
		self.parse_term().map(Expr)
	}

	pub fn parse_term(&mut self) -> Result<TermExpr> {
		let _lhs: FactorExpr = self.parse_factor()?;
		let next: &Token = self.lexer.peek_token()?;
		#[allow(clippy::match_single_binding)]
		match *next {
			_ => (),
		}
		todo!()
	}

	pub fn parse_factor(&mut self) -> Result<FactorExpr> {
		todo!()
	}
}
