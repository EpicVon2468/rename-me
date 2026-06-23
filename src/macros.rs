#[macro_export]
macro_rules! unreachable_ice {
	($msg:expr, $src:ident $(,)?) => {{
		std::hint::cold_path();
		anyhow::bail!(anyhow::Error::context(
			$crate::with_location!(anyhow::anyhow!($msg)),
			$crate::errors::ICE::new($crate::errors::ErrorSource::new(
				"[unimplemented: name of input file]",
				$crate::errors::Phase::$src,
			)),
		));
	}};
}

#[macro_export]
macro_rules! const_num_env {
	($env:literal, $default:literal $(,)?) => {
		const {
			#[inline(always)]
			const fn mapper(value: &str) -> usize {
				// FIXME(const-hack): `str::parse()` is not const yet.
				<usize as std::str::FromStr>::from_str(value).unwrap_or($default)
			}
			let value: usize = option_env!($env).map_or($default, mapper);
			assert!(
				(0..=(isize::MAX.cast_unsigned())).contains(&value),
				concat!(
					"Numeric environment variable `",
					$env,
					"` must be valid for allocation!",
				),
			);
			value
		}
	};
}

#[macro_export]
macro_rules! with_location {
	($err:expr $(,)?) => {{
		anyhow::Error::context(
			anyhow::anyhow!(concat!(
				$crate::location!(),
				" [compiler internal tracking; ignore this line]",
			)),
			$err,
		)
	}};
}

#[macro_export]
macro_rules! location {
	($(,)?) => {
		concat!('[', file!(), ':', line!(), ':', column!(), ']')
	};
}

#[macro_export]
macro_rules! unexpected_token {
	($unexpected:expr $(,)?) => {{
		anyhow::bail!($crate::with_location!(
			$crate::errors::UnexpectedTokenError::new($unexpected),
		));
	}};
}

#[macro_export]
macro_rules! expect_token {
	($(@[unreachable = bail] $(,)?)? $actual:expr, $expected:pat $(, $($name:ident),* )? $(,)?) => {{
		let next: Token = $actual;
		let $expected = next else {
			$crate::unexpected_token!(next);
		};
		$(($($name),*))?
	}};
	(@[unreachable = ICE] $(,)? $actual:expr, $expected:pat $(, $($name:ident),* )? $(,)?) => {{
		let actual = $actual;
		let $expected = actual else {
			$crate::unreachable_ice!(
				format!(concat!("Token {:?} did not match `", stringify!($expected), "`!"), actual),
				Parsing,
			);
		};
		$(($($name),*))?
	}};
}

#[macro_export]
macro_rules! unwrap_identifier {
	($(@[unreachable = bail] $(,)?)? $self:expr $(,)?) => {
		$crate::expect_token!(
			@[unreachable = bail],
			$self.lexer.read_token()?,
			$crate::lexer::Token::Identifier(identifier),
			identifier,
		)
	};
	(@[unreachable = ICE] $(,)? $self:expr $(,)?) => {
		$crate::expect_token!(
			@[unreachable = ICE],
			$self.lexer.read_token(),
			Ok($crate::lexer::Token::Identifier(identifier)),
			identifier,
		)
	};
}
