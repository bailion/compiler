#![cfg_attr(test, feature(no_coverage, trivial_bounds, type_alias_impl_trait))]

pub mod codegen;
pub mod diagnostics;
pub mod id;
pub mod parse;
pub mod ty;
pub mod visitor;
