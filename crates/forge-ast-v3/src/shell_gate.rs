//! AST-level shell command gate (tree-sitter-bash). Ported from
//! F:\NewRepo\crates\forge-ast\src\shell_gate.rs (2026-08-24).

use tree_sitter::{Node, Parser};

/// Verdict from [`evaluate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellGateDecision {
    /// The command passed every check.
    Allow,
    /// The command failed a check; the string is the human-readable reason.
    Deny(String),
}

impl ShellGateDecision {
    /// True iff this verdict is a [`ShellGateDecision::Deny`].
    pub fn is_deny(&self) -> bool {
        matches!(self, ShellGateDecision::Deny(_))
    }
}

const WRAPPERS: &[&str] = &[
    "sudo", "env", "xargs", "nohup", "nice", "timeout", "time",
    "watch", "strace", "ltrace", "parallel", "ionice", "taskset",
    "unbuffer", "script",
];

const DENY_COMMANDS: &[&str] = &[
    "rm", "rmdir", "shred", "mkfs", "dd", "truncate",
    "wrangler", "kubectl",
];

const GIT_DENY_SUBS: &[&str] = &[
    "commit", "push", "rebase", "reset", "clean", "checkout",
    "merge", "cherry-pick", "revert", "stash", "tag", "branch",
];

const SENSITIVE_PATHS: &[&str] = &[
    "/mnt/f", "/mnt/e", "F:\\", "E:\\", "F:/", "E:/",
    "/etc/", "/root/",
];

/// Evaluate one shell command line. Unparseable input allows through —
/// the caller's own trust boundary handles anything the grammar rejects.
pub fn evaluate(command: &str) -> ShellGateDecision {
    let mut parser = Parser::new();
    let language = tree_sitter_bash::LANGUAGE;
    parser.set_language(&language.into()).expect("tree-sitter-bash grammar loads");

    let Some(tree) = parser.parse(command, None) else {
        return ShellGateDecision::Allow;
    };
    walk_for_violations(tree.root_node(), command.as_bytes())
}

fn walk_for_violations(node: Node, src: &[u8]) -> ShellGateDecision {
    match node.kind() {
        "command" => {
            if let Some(d) = check_command(node, src) {
                return d;
            }
        }
        "redirected_statement" | "file_redirect" => {
            if let Some(d) = check_redirections(node, src) {
                return d;
            }
        }
        "command_substitution" => {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    let d = walk_for_violations(child, src);
                    if d.is_deny() {
                        return d;
                    }
                }
            }
        }
        _ => {}
    }
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            let d = walk_for_violations(child, src);
            if d.is_deny() {
                return d;
            }
        }
    }
    ShellGateDecision::Allow
}

fn check_command(node: Node, src: &[u8]) -> Option<ShellGateDecision> {
    let args = collect_command_args(node, src);
    if args.is_empty() {
        return None;
    }
    let cmd = &args[0];
    let (effective_cmd, effective_args) = unwrap_wrappers(&args);

    for deny in DENY_COMMANDS {
        if effective_cmd == *deny {
            return Some(ShellGateDecision::Deny(format!("'{effective_cmd}' is a denied command.")));
        }
    }

    if effective_cmd == "git" {
        if let Some(sub) = effective_args.first() {
            for deny_sub in GIT_DENY_SUBS {
                if sub == deny_sub {
                    return Some(ShellGateDecision::Deny(format!(
                        "'git {sub}' is a denied write operation."
                    )));
                }
            }
        }
    }

    if *cmd == "sudo" {
        return Some(ShellGateDecision::Deny("sudo blocked. No privilege escalation.".into()));
    }

    if effective_cmd == "docker" && effective_args.first().map(String::as_str) == Some("push") {
        return Some(ShellGateDecision::Deny("docker push blocked. No cloud deployment.".into()));
    }
    if effective_cmd == "cargo" && effective_args.first().map(String::as_str) == Some("publish") {
        return Some(ShellGateDecision::Deny("cargo publish blocked. No cloud deployment.".into()));
    }
    if effective_cmd == "npm" && effective_args.first().map(String::as_str) == Some("publish") {
        return Some(ShellGateDecision::Deny("npm publish blocked. No cloud deployment.".into()));
    }

    if matches!(effective_cmd.as_str(), "curl" | "wget" | "scp" | "rsync" | "nc" | "netcat") {
        for arg in &effective_args {
            for path in SENSITIVE_PATHS {
                if arg.contains(path) {
                    return Some(ShellGateDecision::Deny(format!(
                        "network transfer of drive contents blocked: {arg}"
                    )));
                }
            }
        }
    }
    None
}

fn check_redirections(node: Node, src: &[u8]) -> Option<ShellGateDecision> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "file_redirect" {
                let text = node_text(&child, src);
                for path in SENSITIVE_PATHS {
                    if text.contains(path) {
                        return Some(ShellGateDecision::Deny(format!(
                            "redirection to sensitive path blocked: {}",
                            text.trim()
                        )));
                    }
                }
            }
        }
    }
    None
}

fn collect_command_args(node: Node, src: &[u8]) -> Vec<String> {
    let mut args = Vec::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if matches!(
                child.kind(),
                "command_name" | "word" | "string" | "raw_string" | "concatenation" | "simple_expansion"
            ) {
                let text = node_text(&child, src).trim().to_string();
                let text = text.trim_matches('"').trim_matches('\'').to_string();
                if !text.is_empty() {
                    args.push(text);
                }
            }
        }
    }
    args
}

fn unwrap_wrappers(args: &[String]) -> (String, Vec<String>) {
    if args.is_empty() {
        return (String::new(), vec![]);
    }
    let cmd = &args[0];
    if WRAPPERS.iter().any(|w| cmd == w) {
        let rest: Vec<String> =
            args[1..].iter().skip_while(|a| a.starts_with('-') || a.contains('=')).cloned().collect();
        if !rest.is_empty() {
            return unwrap_wrappers(&rest);
        }
    }
    let effective_args = if args.len() > 1 { args[1..].to_vec() } else { vec![] };
    (cmd.clone(), effective_args)
}

fn node_text(node: &Node, src: &[u8]) -> String {
    let start = node.start_byte();
    let end = node.end_byte().min(src.len());
    String::from_utf8_lossy(&src[start..end]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_safe_commands() {
        assert_eq!(evaluate("ls /tmp"), ShellGateDecision::Allow);
        assert_eq!(evaluate("cargo build"), ShellGateDecision::Allow);
        assert_eq!(evaluate("git status"), ShellGateDecision::Allow);
        assert_eq!(evaluate("git log --oneline"), ShellGateDecision::Allow);
        assert_eq!(evaluate("git diff HEAD"), ShellGateDecision::Allow);
    }

    #[test]
    fn denies_rm() {
        assert!(evaluate("rm foo.txt").is_deny());
        assert!(evaluate("rm -rf /tmp/stuff").is_deny());
    }

    #[test]
    fn denies_git_writes() {
        assert!(evaluate("git commit -m test").is_deny());
        assert!(evaluate("git push origin main").is_deny());
        assert!(evaluate("git rebase main").is_deny());
        assert!(evaluate("git reset --hard").is_deny());
        assert!(evaluate("git clean -fd").is_deny());
    }

    #[test]
    fn denies_sudo() {
        assert!(evaluate("sudo apt install foo").is_deny());
        assert!(evaluate("sudo rm /tmp/x").is_deny());
    }

    #[test]
    fn denies_deploy() {
        assert!(evaluate("cargo publish").is_deny());
        assert!(evaluate("docker push myimage").is_deny());
        assert!(evaluate("npm publish").is_deny());
        assert!(evaluate("wrangler deploy").is_deny());
    }

    #[test]
    fn unwraps_env_wrapper() {
        assert!(evaluate("env GIT_AUTHOR=x git push").is_deny());
    }

    #[test]
    fn unwraps_nohup_wrapper() {
        assert!(evaluate("nohup rm -rf /tmp/x &").is_deny());
    }

    #[test]
    fn denies_chained_dangerous() {
        assert!(evaluate("ls && git commit -m x").is_deny());
    }

    #[test]
    fn allows_safe_git_read() {
        assert_eq!(evaluate("git log --oneline -10"), ShellGateDecision::Allow);
        assert_eq!(evaluate("git blame src/main.rs"), ShellGateDecision::Allow);
    }

    #[test]
    fn empty_command() {
        assert_eq!(evaluate(""), ShellGateDecision::Allow);
    }
}
