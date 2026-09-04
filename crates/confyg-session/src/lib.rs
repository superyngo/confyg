//! `confyg-session` — the session layer: overlay the **Document**, dispatch **Setter intents**,
//! and lower them onto `confy-core` `Mutation`s. The compiler stays pure; the ordinal arithmetic
//! and the write path live here.

pub mod lower;
pub mod ordinal;
