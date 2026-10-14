//! Exercise 3: a declarative macro that writes the same code for every enum.
//!
//! Practises: `docs/learning/01-foundation-and-domain.md`, section 6.
//! Mirrors: the `str_enum!` macro in `smoke_domain::types`.
//! Run: `cargo test --test ex03_text_enum`
//!
//! Hint: the pattern already captures the enum name and the `Variant => "text"` pairs. Inside
//! the expansion, repeat a piece of code once per pair with `$( ... ),+` (the `,` is the separator):
//! `match self { $( $name::$variant => $text ),+ }` writes one arm per variant. Replace every
//! `todo!()` in the expansion; `ALL` lists the variants in declaration order. `stringify!($name)`
//! turns the enum's name into text.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownValue {
    pub kind: &'static str,
    pub value: String,
}

#[macro_export]
macro_rules! text_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($variant:ident => $text:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[];

            /// The stable text of this variant.
            pub fn as_str(self) -> &'static str {
                todo!("one match arm per variant")
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::ex03_text_enum::UnknownValue;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                todo!("the variant whose text is `s`, else UnknownValue {{ kind, value }}")
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                todo!("write as_str()")
            }
        }
    };
}
