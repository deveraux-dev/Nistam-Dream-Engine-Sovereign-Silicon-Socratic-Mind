//! Lane RON specs (`*.ron`) + Lane WELD handlers (`*.rs`), one sibling pair
//! per verb. `generated.rs` is produced by `cargo xtask gen-verbs` from the
//! `*.ron` specs + `ORDER.tsv` — never hand-edit it (see `codegen.rs`).

pub mod generated;
pub mod lsp_hover;
pub mod example_empty;
pub mod example_keyed;
pub mod example_verbatim;
