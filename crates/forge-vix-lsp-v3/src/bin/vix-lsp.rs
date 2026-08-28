#![allow(missing_docs)]
//! Standalone VixiScript LSP binary.

fn main() {
    // Start the hand-rolled stdio language server
    let exit_code = forge_vix_lsp_v3::run_stdio();
    std::process::exit(exit_code);
}
