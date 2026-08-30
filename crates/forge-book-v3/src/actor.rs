//! actor.rs — WHO acted, from the OS token, never from a written string.
//!
//! 2026-08-04, the forcing incident: `.forge/board_flips.tsv` carried one row attributed to
//! `seanm` — written by an agent, from `std::env::var("USERNAME")`. Two problems, and the
//! second is the real one. `USERNAME` is an ENVIRONMENT VARIABLE: any process can set it to
//! any string before spawning a child, so it is a claim, not an identity. And the agent runs
//! in Sean's own logon session, so even a truthful read cannot tell the two of them apart.
//! The audit trail said Sean and meant nobody.
//!
//! Two fixes, both here:
//!
//! 1. [`whoami`] reads the SID out of the PROCESS TOKEN (`OpenProcessToken` ->
//!    `GetTokenInformation(TokenUser)`), which no environment variable can forge. The
//!    ForgeAgent/LOTO split then means what it says: run the agent under its own account and
//!    the SID differs, mechanically, with nothing to remember and nothing to trust.
//! 2. [`Attribution::of`] refuses to name Sean on a token alone. A token proves which ACCOUNT
//!    ran; only a live, dated `[SEAN-OK YYYY-MM-DD]` countersign in the text proves the
//!    DECISION was his. T2 signature or it wasn't you.

/// The account a process is actually running as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    /// Display name from the token (`seanm`, `ForgeAgent`). Convenience only.
    pub account: String,
    /// The token's user SID (`S-1-5-21-…`). THE identity — unforgeable by env.
    pub sid: String,
}

impl Actor {
    /// `account/S-1-5-…` — the one-field form for a receipt column.
    pub fn stamp(&self) -> String {
        format!("{}/{}", self.account, self.sid)
    }
}

/// Who is running this process, from the OS token.
///
/// `account` may fall back to `$USERNAME` (a display convenience), but `sid` never does:
/// when the token cannot be read it is the literal `SID-UNAVAILABLE`, which is a stated
/// absence, not a guess. [`Attribution::of`] treats it as such.
pub fn whoami() -> Actor {
    let sid = token_sid().unwrap_or_else(|| "SID-UNAVAILABLE".to_string());
    let account = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string());
    Actor { account, sid }
}

/// WHO the repo may record for an action, and on what evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attribution {
    /// A live `[SEAN-OK YYYY-MM-DD]` countersign accompanied the action — T2.
    Sean {
        /// The identity that carried the countersigned action.
        actor: Actor,
    },
    /// A token, and only a token. Every agent-authored action lands here.
    Machine {
        /// The identity attached to the token-only action.
        actor: Actor,
    },
}

impl Attribution {
    /// Classify an action by the text that carried it plus today's date.
    ///
    /// `today` is passed in rather than read from a clock so the rule is testable, and so a
    /// STALE countersign (yesterday's `[SEAN-OK]` pasted forward) never grants authority —
    /// the same law `forge_daemon::gate::countersign_on` enforces at the hook door.
    pub fn of(text: &str, today: &str) -> Attribution {
        let actor = whoami();
        if countersigned_on(text, today) {
            Attribution::Sean { actor }
        } else {
            Attribution::Machine { actor }
        }
    }

    /// The receipt column: `sean:<stamp>` or `machine:<stamp>`. Never a bare name — a bare
    /// name is what could not distinguish Sean from the agent in the first place.
    pub fn stamp(&self) -> String {
        match self {
            Attribution::Sean { actor } => format!("sean:{}", actor.stamp()),
            Attribution::Machine { actor } => format!("machine:{}", actor.stamp()),
        }
    }

    /// Is this a T2 (Sean) signature? The one question a delete/waiver gate should ask.
    pub fn is_sean(&self) -> bool {
        matches!(self, Attribution::Sean { .. })
    }
}

/// Is a LIVE `[SEAN-OK <today>]` in the text? Bare `[SEAN-OK]` does NOT count: an undated
/// token is copy-pasteable forever, which makes it a password, not a signature.
pub fn countersigned_on(text: &str, today: &str) -> bool {
    text.contains(&format!("[SEAN-OK {today}]"))
}

// ── OS token read ────────────────────────────────────────────────────────────────────────
// Declared inline against advapi32/kernel32 rather than pulling a `windows` crate into
// forge-book, which is a leaf below the brain layer and carries no platform dep today.

#[cfg(windows)]
mod win {
    #[link(name = "advapi32")]
    extern "system" {
        pub fn OpenProcessToken(process: isize, access: u32, token: *mut isize) -> i32;
        pub fn GetTokenInformation(
            token: isize,
            class: i32,
            info: *mut u8,
            len: u32,
            ret_len: *mut u32,
        ) -> i32;
        pub fn ConvertSidToStringSidW(sid: *mut u8, out: *mut *mut u16) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetCurrentProcess() -> isize;
        pub fn CloseHandle(h: isize) -> i32;
        pub fn LocalFree(p: *mut u16) -> *mut u16;
    }
    pub const TOKEN_QUERY: u32 = 0x0008;
    pub const TOKEN_USER: i32 = 1;
}

/// The current process token's user SID as `S-1-5-21-…`, or `None` if the OS refused.
#[cfg(windows)]
#[allow(unsafe_code)]
fn token_sid() -> Option<String> {
    // SAFETY: every call is checked for the Win32 zero-is-failure return before its output
    // is read; the handle and the LocalAlloc'd string are released on every exit path.
    unsafe {
        let mut token: isize = 0;
        if win::OpenProcessToken(win::GetCurrentProcess(), win::TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        // TOKEN_USER is { SID_AND_ATTRIBUTES { PSID, u32 } } followed by the SID body the
        // pointer aims into, so the buffer must hold both. Ask for the size, then fill it.
        let mut need: u32 = 0;
        win::GetTokenInformation(token, win::TOKEN_USER, std::ptr::null_mut(), 0, &mut need);
        if need == 0 {
            win::CloseHandle(token);
            return None;
        }
        let mut buf = vec![0u8; need as usize];
        let ok =
            win::GetTokenInformation(token, win::TOKEN_USER, buf.as_mut_ptr(), need, &mut need);
        win::CloseHandle(token);
        if ok == 0 {
            return None;
        }
        // First field of TOKEN_USER is the PSID.
        let psid = std::ptr::read_unaligned(buf.as_ptr() as *const *mut u8);
        if psid.is_null() {
            return None;
        }
        let mut wide: *mut u16 = std::ptr::null_mut();
        if win::ConvertSidToStringSidW(psid, &mut wide) == 0 || wide.is_null() {
            return None;
        }
        let mut n = 0usize;
        while *wide.add(n) != 0 {
            n += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(wide, n));
        win::LocalFree(wide);
        Some(s)
    }
}

#[cfg(not(windows))]
fn token_sid() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: BOARD-ACTOR-TOKEN]
    /// The SID comes from the token, so poisoning the environment cannot move it.
    ///
    /// This is the 08-04 incident as an executable test: `USERNAME` is writable by the
    /// process, and the old attribution read exactly that.
    #[test]
    #[cfg(windows)]
    fn a_forged_username_cannot_move_the_sid() {
        let real = whoami();
        assert!(real.sid.starts_with("S-1-"), "token SID must be a real SID, got {}", real.sid);
        // SAFETY: single-threaded test scope; the value is restored before returning.
        let prior = std::env::var("USERNAME").ok();
        std::env::set_var("USERNAME", "Administrator");
        let forged = whoami();
        match prior {
            Some(p) => std::env::set_var("USERNAME", p),
            None => std::env::remove_var("USERNAME"),
        }
        assert_eq!(forged.sid, real.sid, "the SID is token-derived — no env var reaches it");
        assert_eq!(forged.account, "Administrator", "and the NAME is exactly the forgeable half");
    }

    // [BOARD: BOARD-ACTOR-TOKEN]
    /// T2 or it wasn't you: a token alone never attributes to Sean.
    #[test]
    fn only_a_live_countersign_attributes_to_sean() {
        let today = "2026-08-04";
        assert!(!Attribution::of("waived: font CI box", today).is_sean(), "prose is not a signature");
        assert!(
            !Attribution::of("[SEAN-OK]", today).is_sean(),
            "an undated token is a password, not a signature"
        );
        assert!(
            !Attribution::of("[SEAN-OK 2026-08-03]", today).is_sean(),
            "yesterday's countersign pasted forward is not today's decision"
        );
        assert!(Attribution::of("ship it [SEAN-OK 2026-08-04]", today).is_sean());
    }

    // [BOARD: BOARD-ACTOR-TOKEN]
    /// A machine stamp carries the SID, so the row can be told apart from Sean's later.
    #[test]
    fn a_machine_stamp_names_the_token_not_a_word() {
        let s = Attribution::of("no countersign here", "2026-08-04").stamp();
        assert!(s.starts_with("machine:"), "{s}");
        assert!(s.contains('/'), "the stamp carries account/SID, not a bare name: {s}");
    }
}
