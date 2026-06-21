macro_rules! float_type {
	($__float:ident, $num_bits:literal, $aka:literal, $type_fn:ident $(,)?) => {
		crate::zst_singleton!(
			$__float,
			$type_fn,
			#[doc = concat!("Singleton struct representation of a ", $num_bits, "-bit [IEEE-754](https://standards.ieee.org/ieee/754/6210/) floating-point number type.  Also known as the ", $aka, " type.")]
			#[doc = ""]
			#[doc = "It is recommended to use the exposed functions (i.e. [`Self::instance`]) as opposed to the internal [ZST](https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts) singleton, as this may be refactored in future."]
			#[doc = ""]
			#[doc = "# Examples"]
			#[doc = ""]
			#[doc = "```"]
			#[doc = concat!("let _f", $num_bits, "_type = ", stringify!($__float), "::instance();")]
			#[doc = "```"]
		);
	};
}

float_type!(__16BitFloatingPoint, 16, "`half`", f16_type);
float_type!(__16BitBrainFloatingPoint, 16, "`bfloat16`", bf16_type);
float_type!(__32BitFloatingPoint, 32, "`float` or `binary32`", f32_type);
float_type!(__64BitFloatingPoint, 64, "`double` or `binary64`", f64_type);
float_type!(__128BitFloatingPoint, 128, "`binary128`", f128_type);
