use std::fmt::{Debug, Formatter};

use crate::parser::fn_attrs::{Attributes, fmt_attributes};
use crate::parser::modifiers::{Modifiers, fmt_modifiers};
use crate::types::__Type;

#[must_use]
pub struct FunctionDeclaration {
	modifiers: Modifiers,
	attributes: Attributes,
	identifier: String,
	// HashMap is not ordered
	parameters: Vec<(String, Box<dyn __Type>)>,
	return_type: Box<dyn __Type>,
}

impl FunctionDeclaration {
	pub fn new(
		modifiers: Modifiers,
		attributes: Attributes,
		mut identifier: String,
		parameters: Vec<(String, Box<dyn __Type>)>,
		return_type: Box<dyn __Type>,
	) -> Self {
		identifier.push('\0');
		Self {
			modifiers,
			attributes,
			identifier,
			parameters,
			return_type,
		}
	}

	#[must_use]
	pub const fn modifiers(&self) -> Modifiers {
		self.modifiers
	}

	#[must_use]
	pub const fn attributes(&self) -> Attributes {
		self.attributes
	}

	#[must_use]
	pub const fn identifier(&self) -> &str {
		self.identifier.as_str()
	}

	#[must_use]
	pub const fn parameters(&self) -> &[(String, Box<dyn __Type>)] {
		&self.parameters
	}

	#[must_use]
	pub const fn return_type(&self) -> &dyn __Type {
		&self.return_type
	}
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
