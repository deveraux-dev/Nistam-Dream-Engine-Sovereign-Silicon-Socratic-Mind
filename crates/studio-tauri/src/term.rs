//! ConPTY terminal organ — portable-pty (donor: shell/src/pty.rs, the RACE
//! and BELL scars carried verbatim) feeding forge_tui_v3::vt::Terminal (the
//! one VT500 home); a pump thread emits the dirty grid as run-length rows.

use forge_tui_v3::vt::Terminal;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

/// Session setup pwsh runs before the boot line: UTF-8 both ways (mis-decode
/// shatters glyphs), PSReadLine bell + prediction off (fights raw input), then
/// a user/machine env re-import so keys set after the GUI parent launched
/// (gcloud/gemini setx, System Settings) reach this pane without a relaunch —
/// PATH merges (process+user+machine), every other user var wins fresh.
const PWSH_SETUP: &str = "[Console]::OutputEncoding=[Text.Encoding]::UTF8;$OutputEncoding=[Text.Encoding]::UTF8;Set-PSReadLineOption -BellStyle None -PredictionSource None;$__u=[Environment]::GetEnvironmentVariables('User');$__m=[Environment]::GetEnvironmentVariables('Machine');foreach($__k in $__u.Keys){if($__k -ine 'Path'){Set-Item -Path (\"env:\"+$__k) -Value $__u[$__k]}};$env:Path=(@($env:Path,$__u['Path'],$__m['Path']) -ne $null -join ';')";

/// A live pseudo-console hosting a shell process.
pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    out_rx: Receiver<Vec<u8>>,
    in_tx: Sender<Vec<u8>>,
}

impl Pty {
    /// Open a ConPTY and spawn pwsh with `boot` chained onto the session
    /// setup via `-Command` (typed-in boot lines lose the PSReadLine flush
    /// race — the donor's 2026-08-03 scar).
    pub fn spawn_boot(cols: u16, rows: u16, boot: &str) -> Result<Self, String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| format!("openpty: {e}"))?;

        let mut cmd = CommandBuilder::new("pwsh.exe");
        // The glass renders 24-bit per cell and forge-tui's VT parses
        // 16/256/truecolour SGR — declare it, or env-probing CLIs warn.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        // The sky palette rides the pane's environment (Sean 2026-08-26: "the
        // ANSI colours is what I wanted my different verbs set to in env") —
        // FORGE_INK_<TAG>=#RRGGBB, one home forge_core_v3::sky.
        {
            use forge_core_v3::sky::{Brightness, Spectral};
            let hex = |[r, g, b, _a]: [u8; 4]| format!("#{r:02X}{g:02X}{b:02X}");
            for s in [
                Spectral::DeepWinter, Spectral::BoneStar, Spectral::Frost,
                Spectral::AskiyGold, Spectral::TheForge, Spectral::Wisakedjak,
                Spectral::Wanderer, Spectral::TheDistant, Spectral::Meskanaw,
            ] {
                cmd.env(format!("FORGE_INK_{}", s.label()), hex(s.rgba()));
            }
            for b in [
                Brightness::SpiritFire, Brightness::GuideStar,
                Brightness::AncestorLight, Brightness::TheForgotten,
            ] {
                cmd.env(format!("FORGE_INK_{}", b.label()), hex(b.rgba()));
            }
        }
        let arg = if boot.trim().is_empty() {
            PWSH_SETUP.to_string()
        } else {
            format!("{PWSH_SETUP};{}", boot.trim())
        };
        cmd.args(["-NoLogo", "-NoExit", "-Command", arg.as_str()]);

        let mut reader = pair.master.try_clone_reader().map_err(|e| format!("clone reader: {e}"))?;
        let mut writer = pair.master.take_writer().map_err(|e| format!("take writer: {e}"))?;
        let child = pair.slave.spawn_command(cmd).or_else(|_| {
            let mut cmd_fallback = CommandBuilder::new("powershell.exe");
            cmd_fallback.env("TERM", "xterm-256color");
            cmd_fallback.env("COLORTERM", "truecolor");
            cmd_fallback.args(["-NoLogo", "-NoExit", "-Command", arg.as_str()]);
            pair.slave.spawn_command(cmd_fallback)
        }).map_err(|e| format!("spawn shell (pwsh/powershell): {e}"))?;
        drop(pair.slave);

        let (out_tx, out_rx) = channel::<Vec<u8>>();
        let (in_tx, in_rx) = channel::<Vec<u8>>();

        std::thread::Builder::new()
            .name("tauri-pty-reader".into())
            .spawn(move || {
                use std::io::Read;
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if out_tx.send(buf[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                    }
                }
            })
            .map_err(|e| format!("reader thread: {e}"))?;

        std::thread::Builder::new()
            .name("tauri-pty-writer".into())
            .spawn(move || {
                use std::io::Write;
                while let Ok(bytes) = in_rx.recv() {
                    if writer.write_all(&bytes).is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
            })
            .map_err(|e| format!("writer thread: {e}"))?;

        Ok(Self { master: pair.master, _child: child, out_rx, in_tx })
    }

    /// Non-blocking pull of one chunk of shell output, if any.
    pub fn try_read(&mut self) -> Option<Vec<u8>> {
        self.out_rx.try_recv().ok()
    }

    /// Queue bytes to the shell's stdin.
    pub fn write(&self, data: &[u8]) {
        let _ = self.in_tx.send(data.to_vec());
    }

    /// Resize the pseudo-console.
    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
    }
}

/// One run of same-pen cells: text, fg RGBA, bg RGBA.
#[derive(Serialize, Clone)]
pub struct Run(pub String, pub u32, pub u32);

/// One published terminal frame — run-length rows plus the cursor cell.
/// `view`/`depth` = scrollback viewport offset and retained history rows,
/// so the glass can draw a scroll chip and page-clamp without a round trip.
#[derive(Serialize, Clone)]
pub struct TermFrame {
    pub cols: u16,
    pub rows: u16,
    pub cursor: (u32, u32),
    pub grid: Vec<Vec<Run>>,
    pub alt: bool,
    pub view: u32,
    pub depth: u32,
}

/// The live session: ConPTY + the VT500 screen it drives.
pub struct TermSession {
    pub pty: Pty,
    pub term: Terminal,
    pub cols: u16,
    pub rows: u16,
}

/// Serialize the visible grid as run-length rows.
pub fn frame(session: &TermSession) -> TermFrame {
    let (cols, rows) = (session.cols, session.rows);
    let mut grid = Vec::with_capacity(rows as usize);
    for y in 0..rows as u32 {
        let mut row: Vec<Run> = Vec::new();
        for x in 0..cols as u32 {
            let c = session.term.visible_cell(x, y);
            let ch = char::from_u32(c.glyph).unwrap_or(' ');
            match row.last_mut() {
                Some(run) if run.1 == c.fg && run.2 == c.bg => run.0.push(ch),
                _ => row.push(Run(ch.to_string(), c.fg, c.bg)),
            }
        }
        grid.push(row);
    }
    TermFrame {
        cols,
        rows,
        cursor: session.term.cursor(),
        grid,
        alt: session.term.alt_active(),
        view: session.term.view_offset(),
        depth: session.term.scrollback_len() as u32,
    }
}

/// Per-push phrase ceiling: a push is a phrase, never a wall of sound.
const NOTES_PER_PUSH_MAX: usize = 16;

/// One sung note for the glass: frequency mHz, duration ms, MIDI note.
#[derive(Serialize, Clone, Copy)]
pub struct SungNote {
    /// Frequency in millihertz.
    pub mhz: u32,
    /// Duration in milliseconds.
    pub ms: u16,
    /// MIDI note number.
    pub midi: u8,
}

/// VT escape-walk state for the ear.
enum EscState {
    /// Plain bytes.
    Ground,
    /// After ESC.
    Esc,
    /// Inside CSI, until a final byte.
    Csi,
    /// Inside OSC, until the next ESC.
    Osc,
}

/// The dock's ear: assembles words out of the byte stream (escape sequences
/// skipped) and routes each through the one forge_harmonics home into a
/// pentatonic-keyed note.
pub struct WordEar {
    esc: EscState,
    word: Vec<u8>,
    notes: Vec<SungNote>,
    router: forge_harmonics::InteractiveHarmonicRouter,
}

impl WordEar {
    /// A silent ear with nothing pending.
    pub fn new() -> Self {
        Self {
            esc: EscState::Ground,
            word: Vec::new(),
            notes: Vec::new(),
            router: forge_harmonics::InteractiveHarmonicRouter::new(
                forge_harmonics::CamelotKey::DEFAULT_8A,
                forge_harmonics::HarmonicPreset::Pentatonic12Tet,
            ),
        }
    }

    /// Close the word in flight, voicing it if there is one.
    fn break_word(&mut self) {
        if self.word.is_empty() {
            return;
        }
        let vn = self.router.route_word(&self.word);
        if self.notes.len() < NOTES_PER_PUSH_MAX {
            self.notes.push(SungNote { mhz: vn.freq_mhz, ms: vn.duration_ms, midi: vn.midi_note });
        }
        self.word.clear();
    }

    /// Feed one raw pty chunk — exactly what the screen sees.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.router.feed_chunk(chunk);
        for &b in chunk {
            match self.esc {
                EscState::Ground => {
                    if b == 0x1b {
                        self.break_word();
                        self.esc = EscState::Esc;
                    } else if b.is_ascii_alphanumeric() || b == b'_' {
                        if self.word.len() < 64 {
                            self.word.push(b);
                        }
                    } else {
                        self.break_word();
                    }
                }
                EscState::Esc => {
                    self.esc = match b {
                        b'[' => EscState::Csi,
                        b']' => EscState::Osc,
                        _ => EscState::Ground,
                    };
                }
                EscState::Csi => {
                    if (0x40..=0x7e).contains(&b) {
                        self.esc = EscState::Ground;
                    }
                }
                EscState::Osc => {
                    // BEL terminates OSC here: this ear hears the RAW stream,
                    // and ConPTY titles end in BEL constantly.
                    if b == 0x1b {
                        self.esc = EscState::Esc;
                    } else if b == 0x07 {
                        self.esc = EscState::Ground;
                    }
                }
            }
        }
    }

    /// Drain the pending phrase.
    pub fn take_notes(&mut self) -> Vec<SungNote> {
        self.notes.drain(..).collect()
    }
}

/// Pump loop: drain pty bytes into the VT machine, answer DSR queries, and
/// emit a `term-grid` frame whenever the grid went dirty (~30 Hz cap).
pub fn pump<R: tauri::Runtime>(app: tauri::AppHandle<R>, session: Arc<Mutex<Option<TermSession>>>) {
    let mut ear = WordEar::new();
    loop {
        let mut dirty = false;
        {
            let mut guard = session.lock().unwrap_or_else(|p| p.into_inner());
            let Some(s) = guard.as_mut() else { break };
            for _ in 0..64 {
                let Some(bytes) = s.pty.try_read() else { break };
                ear.feed(&bytes);
                s.term.feed(&bytes);
                dirty = true;
            }
            if let Some(reply) = s.term.take_reply() {
                s.pty.write(&reply);
            }
            if dirty || s.term.grid().dirty {
                s.term.grid_mut().dirty = false;
                let f = frame(s);
                drop(guard);
                let _ = app.emit("term-grid", &f);
            }
        }
        let phrase = ear.take_notes();
        if !phrase.is_empty() {
            let _ = app.emit("term-notes", &phrase);
        }
        std::thread::sleep(Duration::from_millis(33));
    }
}
