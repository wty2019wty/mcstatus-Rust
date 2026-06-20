//! MOTD (Message of the Day) parsing and transformation.
//!
//! Parses raw MOTD strings or JSON dicts into structured component lists,
//! and transforms them into various output formats (plain text, HTML, ANSI,
//! Minecraft section-sign format).

pub mod components;
pub mod simplify;
pub mod transform;

mod motd_impl;

pub use motd_impl::Motd;
