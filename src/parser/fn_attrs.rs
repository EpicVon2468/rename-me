use std::fmt::{Debug, Formatter, Result};
use std::ops::BitOrAssign;

macro_rules! is_attr {
	($check_name:ident, $attr:expr $(,)?) => {
		pub const fn $check_name(&self) -> bool {
			self.state & $attr != 0
		}
	};
}

#[derive(PartialEq, Eq)]
#[derive_const(Default)]
pub struct Attributes {
	state: u8,
	attached_data: Vec<Attribute>,
}

impl Attributes {
	is_attr!(is_cold, ATTR_COLD);
	is_attr!(is_hot, ATTR_HOT);
	is_attr!(is_strictfp, ATTR_STRICTFP);
	is_attr!(is_try_inline, ATTR_TRY_INLINE);
	is_attr!(is_force_inline, ATTR_FORCE_INLINE);
	is_attr!(is_method, ATTR_METHOD);

	#[must_use]
	pub fn purity(&self) -> Option<bool> {
		if self.state & ATTR_PURITY == 0 {
			return None;
		};
		let data: Option<&Attribute> = self
			.attached_data
			.iter()
			.find(|data: &&Attribute| matches!(data, Attribute::Purity(_)));
		debug_assert!(data.is_some());
		let Some(&Attribute::Purity(purity)) = data else {
			std::hint::cold_path();
			unreachable!(
				"Data was marked as present in attribute state, but was not actually present!",
			);
		};
		Some(purity)
	}

	#[must_use]
	pub const fn contains(&self, attribute: &Attribute) -> bool {
		self.state & attribute.discriminant() != 0
	}

	#[must_use]
	pub fn with_attr(mut self, attribute: Attribute) -> Self {
		self |= attribute;
		self
	}
}

impl BitOrAssign<Attribute> for Attributes {
	fn bitor_assign(&mut self, rhs: Attribute) {
		self.state |= rhs.discriminant();
		if rhs.carries_data() {
			self.attached_data.push(rhs);
		};
	}
}

impl BitOrAssign<Self> for Attributes {
	fn bitor_assign(&mut self, rhs: Self) {
		if rhs.state == 0 {
			return;
		};
		self.state |= rhs.state;
		for attr in rhs.attached_data {
			if !self.attached_data.contains(&attr) {
				self.attached_data.push(attr);
			};
		}
	}
}

impl Debug for Attributes {
	fn fmt(&self, fmt: &mut Formatter<'_>) -> Result {
		fmt.debug_struct("Attributes")
			.field("is_cold", &self.is_cold())
			.field("is_hot", &self.is_hot())
			.field("is_strictfp", &self.is_strictfp())
			.field("is_try_inline", &self.is_try_inline())
			.field("is_force_inline", &self.is_force_inline())
			.field("is_method", &self.is_method())
			.field("explicit_purity", &self.purity())
			.finish()
	}
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Attribute {
	/// [`llvm.cold`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=cold,-This)
	Cold = ATTR_COLD,
	/// [`llvm.hot`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=hot,-This)
	Hot = ATTR_HOT,
	/// [`llvm.strictfp`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=strictfp)
	Strictfp = ATTR_STRICTFP,
	/// [`llvm.inlinehint`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=inlinehint)
	TryInline = ATTR_TRY_INLINE,
	/// [`llvm.alwaysinline`](https://llvm.org/docs/LangRef.html#function-attributes:~:text=alwaysinline,-This)
	ForceInline = ATTR_FORCE_INLINE,
	Method(String) = ATTR_METHOD,
	/// Whether a function is effectful (impure) or effectless (pure).
	Purity(bool) = ATTR_PURITY,
}

impl Attribute {
	#[must_use]
	pub const fn carries_data(&self) -> bool {
		matches!(self, Self::Method(_) | Self::Purity(_))
	}

	// Taken from `std::mem::discriminant`'s docs.
	#[must_use]
	pub const fn discriminant(&self) -> u8 {
		// SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
		// between `repr(C)` structs, each of which has the `u8` discriminant as its first
		// field, so we can read the discriminant without offsetting the pointer.
		unsafe { *<*const _>::from(self).cast::<u8>() }
	}
}

pub const ATTR_COLD: u8 = 0b0000_0001;
pub const ATTR_HOT: u8 = 0b0000_0010;
pub const ATTR_STRICTFP: u8 = 0b0000_0100;
pub const ATTR_TRY_INLINE: u8 = 0b0000_1000;
pub const ATTR_FORCE_INLINE: u8 = 0b0001_0000;
pub const ATTR_METHOD: u8 = 0b0010_0000;
pub const ATTR_PURITY: u8 = 0b0100_0000;
