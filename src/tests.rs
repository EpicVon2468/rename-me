#![cfg(test)]

use std::io::Read;

use crate::{
	parser::{
		Parser,
		TopLevel,
		fn_attrs::{Attribute, Attributes},
		function::FunctionDeclaration,
		modifiers::{MOD_CONSTANT, Modifiers},
	},
	types::{__Void, integers::__32BitInteger},
};

#[test]
pub fn function_parse_test() {
	{
		let mut input: &[u8] = b"funct main();";
		let mut parser: Parser = Parser::from(&mut input as &mut dyn Read);
		let expected: FunctionDeclaration = FunctionDeclaration::new(
			Modifiers::default(),
			Attributes::default(),
			String::from("main"),
			Vec::default(),
			Box::new(__Void::instance()),
		);
		assert_eq!(
			TopLevel::Function(expected),
			parser.parse().expect("Test failed!"),
		);
	};
	{
		let mut input: &[u8] = b"#(hot, strictfp, force_inline) const funct main(): i32;";
		let mut parser: Parser = Parser::from(&mut input as &mut dyn Read);
		let expected: FunctionDeclaration = FunctionDeclaration::new(
			MOD_CONSTANT,
			Attributes::default()
				.with_attr(Attribute::Hot)
				.with_attr(Attribute::Strictfp)
				.with_attr(Attribute::ForceInline),
			String::from("main"),
			Vec::default(),
			Box::new(__32BitInteger::signed()),
		);
		assert_eq!(
			TopLevel::Function(expected),
			parser.parse().expect("Test failed!"),
		);
	};
}
