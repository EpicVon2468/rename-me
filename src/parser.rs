use std::collections::HashMap;

use anyhow::Result;

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

use attrs::Attributes;

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
	/// `blockExpr : ( CONSTANT? UNSAFE? )? OPEN_BRACE stmt* expr CLOSE_BRACE ;`
	Block(Attributes, Vec<Stmt>, Box<Self>),
	IntegerLiteral(i32),
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
		let _ = dbg!(self.parse_term_expr()?);
		Ok(())
	}

	pub fn parse_term_expr(&mut self) -> Result<Expr> {
		let mut result: Expr = self.parse_factor_expr()?;
		loop {
			let op: Term = match *self.lexer.peek_token()? {
				Token::Plus => Term::Add,
				Token::Minus => Term::Sub,
				_ => break,
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
			let op: Factor = match *self.lexer.peek_token()? {
				Token::Asterisk => Factor::Mul,
				Token::Slash => Factor::Div,
				_ => break,
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
		if op.is_some() {
			// Eat `op`.
			let _ = self.lexer.read_token();
		};
		let rhs: Expr = self.parse_function_expr()?;
		Ok(match op {
			Some(unary) => Expr::Unary(unary, rhs.into()),
			None => rhs,
		})
	}

	pub fn parse_function_expr(&mut self) -> Result<Expr> {
		todo!()
	}
}
