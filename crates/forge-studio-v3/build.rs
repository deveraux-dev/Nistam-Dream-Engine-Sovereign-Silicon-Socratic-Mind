use std::path::PathBuf;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let kit_path = PathBuf::from(&manifest_dir).join("panels/studio_face.kit.vixi");
    let repo_root = PathBuf::from(&manifest_dir).parent().unwrap().parent().unwrap().to_path_buf();

    println!("cargo:rerun-if-changed={}", kit_path.display());

    let src = match std::fs::read_to_string(&kit_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {}: {}", kit_path.display(), e);
            std::process::exit(1);
        }
    };

    let vp = forge_vix_v3::ir::IrRect { min_x: 0, min_y: 0, max_x: 1400_000, max_y: 900_000 };
    let mut html = match forge_vix_v3::compile_kit_to_html(&src, "13FORGE STUDIO — 3 Gemmas", vp) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("failed to compile studio_face.kit.vixi: {:?}", e);
            std::process::exit(1);
        }
    };

    // Extract slot info (names + source attributes) in lowering order (deterministic).
    let slot_info = extract_slot_info(&src);

    // Post-process: inject data-vixi-id, palette CSS, and text bindings.
    html = compose_webview_layer(html, &slot_info);

    let celestial_js = generate_celestial_js(&repo_root);
    let mud_js = generate_mud_js(&repo_root);
    let builder_js = generate_builder_js(&repo_root);
    let badges_js = generate_badges_init_js();
    let chat_js = generate_chat_js();
    let pty_hearth_js = generate_pty_hearth_js();

    let inject_script = format!(
        "<script>\n{}\n{}\n{}\n{}\n{}\n{}\n</script>",
        celestial_js, mud_js, builder_js, badges_js, chat_js, pty_hearth_js
    );
    html.push_str("\n");
    html.push_str(&inject_script);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(&out_dir).join("studio_face.html");
    match std::fs::write(&out_path, &html) {
        Ok(_) => println!("cargo:warning=Compiled studio_face.kit.vixi + organs to {}", out_path.display()),
        Err(e) => {
            eprintln!("failed to write {}: {}", out_path.display(), e);
            std::process::exit(1);
        }
    }
}

/// Slot info: (slot_name, source_attr_if_any).
/// source is the `source=` attribute value that JS selectors will target.
#[derive(Clone)]
struct SlotInfo {
    name: String,
    source: Option<String>,
}

/// Extract slot definitions from the kit in lowering order.
/// Captures both slot path and source attribute (if present) for JS binding.
fn extract_slot_info(kit_src: &str) -> Vec<SlotInfo> {
    let mut slots = Vec::new();
    for line in kit_src.lines() {
        let line = line.trim();
        if line.starts_with("slot ") {
            let mut parts = line.split_whitespace();
            if let Some(name) = parts.nth(1) {
                let source = line.split("source=")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .map(|s| s.to_string());
                slots.push(SlotInfo {
                    name: name.to_string(),
                    source,
                });
            }
        }
    }
    slots
}

/// Compose the webview content layer: inject data-vixi-id, Molten palette CSS, and text bindings.
/// Kit emits geometry only; build.rs is the content compositor.
fn compose_webview_layer(mut html: String, slot_info: &[SlotInfo]) -> String {
    // Inject Molten palette CSS before closing </style>.
    let molten_css = r#"
#vp{background:#0A0705!important}
#vp>div{background:transparent!important;color:#F7E9D2;font:13px Consolas,monospace;overflow:hidden}
#vp>div[data-vixi-id="message_log"],#vp>div[data-vixi-id="prompt_input"],#vp>div[data-vixi-id="hearth_body"]{background:#1A0F09!important;border:1px solid #3a2415}
#vp>div[data-vixi-id$="_title"],#vp>div[data-vixi-id$="_status"]{color:#FF6A1A}
#vp>div[data-vixi-id$="_stats"],#vp>div[data-vixi-id$="_tag"],#vp>div[data-vixi-id$="_body"],#vp>div[data-vixi-id*="footer"]{color:#B08A63}
#vp>div[data-vixi-id="send_prompt"],#vp>div[data-vixi-id*="wake"]{background:#FF6A1A!important;color:#0A0705;text-align:center;cursor:pointer}
#vp>div[data-vixi-id$="_label"]{color:#C8791E}
.t{color:#F7E9D2!important}
"#;

    if let Some(style_end) = html.find("</style>") {
        html.insert_str(style_end, molten_css);
    }

    // Post-process divs: add data-vixi-id based on slot order.
    // The emitter creates divs in deterministic lowering order matching slot_info order.
    let mut div_count = 0;
    let mut result = String::new();
    let mut chars = html.chars().peekable();
    let mut in_div = false;
    let mut div_attrs = String::new();

    while let Some(ch) = chars.next() {
        result.push(ch);

        if ch == '<' && chars.peek() == Some(&'d') {
            // Start of <div
            in_div = true;
            div_attrs.clear();
        } else if in_div && ch == '>' {
            // End of <div ...>, inject data-vixi-id
            in_div = false;
            // Don't inject ID for the root #vp div itself
            if div_count < slot_info.len() && !div_attrs.contains("id=\"vp\"") {
                // Use source attribute if available (for JS selectors), else use slot name
                let id = slot_info[div_count]
                    .source
                    .as_ref()
                    .unwrap_or(&slot_info[div_count].name);
                result.insert_str(result.len() - 1, &format!(r#" data-vixi-id="{}""#, id));
                div_count += 1;
            } else if !div_attrs.contains("id=\"vp\"") {
                div_count += 1;
            }
        } else if in_div {
            div_attrs.push(ch);
        }
    }

    // Static text pass: fill labeled empty divs (kit is a GPU spec; text lives here).
    const TEXTS: &[(&str, &str)] = &[
        ("window_title", "13FORGE STUDIO — NISTAM DREAM ENGINE"),
        ("daemon_status", "● OFFLINE"),
        ("hero_title", "3 GEMMAS"),
        ("hero_tag", "sovereign silicon · one machine · offline forever"),
        ("root.hero.theme", "MOLTEN ⇄ PERMAFROST"),
        ("root.hero.wake.label", "▶ WAKE TRIAD"),
        ("chat_info", "TRIAD CHAT · dose:4 · warm ON · 466ms TTFT"),
        ("root.main.chat.input.send.label", "SEND"),
        ("baby_status", "●"),
        ("baby_label", "BABY — 2B RENDER"),
        ("baby_stats", "82.5 tok/s · 405 MB"),
        ("mama_status", "●"),
        ("mama_label", "MAMA — 4B ASSIST"),
        ("mama_stats", "54.9 tok/s · 642 MB"),
        ("papa_status", "●"),
        ("papa_label", "PAPA — 9B INTENT"),
        ("papa_stats", "21-23 tok/s · 1.6 GB"),
        ("hearth_title", "HEARTH  [FORGE] [BOOT] [LIVE] — 3 ConPTY tabs (phase C)"),
        ("root.footer", "Ready — the triad is this machine.   ·   one bin · v3 · S13"),
    ];
    for (id, text) in TEXTS {
        let empty = format!(r#"data-vixi-id="{}"></div>"#, id);
        let filled = format!(r#"data-vixi-id="{}">{}</div>"#, id, text);
        result = result.replacen(&empty, &filled, 1);
    }

    result
}

fn generate_celestial_js(repo_root: &PathBuf) -> String {
    let hyg_path = repo_root.join("shell/assets/hyg_baked.bin");
    let _star_count = match std::fs::metadata(&hyg_path) {
        Ok(m) => {
            (m.len() as i32 - 32) / 17
        }
        Err(_) => 0,
    };

    r#"(function setupCelestial() {
  var stars = [];
  var title = 'CELESTIAL hyg bake pending';
  var body = 'procedural starfield (phase C)';
  document.querySelectorAll('[data-vixi-id*="celestial_title"]').forEach(el => { el.textContent = title; });
  document.querySelectorAll('[data-vixi-id*="celestial_body"]').forEach(el => { el.textContent = body; });
})();"#.to_string()
}

fn generate_mud_js(_repo_root: &PathBuf) -> String {
    r#"(function setupMud() {
  var title = 'MUD CONSOLE offline — viewport';
  var body = 'wire.rs integration phase C';
  document.querySelectorAll('[data-vixi-id*="mud_title"]').forEach(el => { el.textContent = title; });
  document.querySelectorAll('[data-vixi-id*="mud_body"]').forEach(el => { el.textContent = body; });
})();"#.to_string()
}

fn generate_builder_js(repo_root: &PathBuf) -> String {
    let zones_path = repo_root.join("assets/ironroot/ironroot_world_systems_bundle.v2.merged.json");
    let has_zones = zones_path.exists();

    let body_text = if has_zones {
        "zone content loaded — grid render phase C"
    } else {
        "no zone content on disk"
    };

    format!(
        r#"(function setupBuilder() {{
  var title = 'WORLD BUILDER';
  var body = '{}';
  document.querySelectorAll('[data-vixi-id*="builder_title"]').forEach(el => {{ el.textContent = title; }});
  document.querySelectorAll('[data-vixi-id*="builder_body"]').forEach(el => {{ el.textContent = body; }});
}})();"#,
        body_text
    )
}

fn generate_badges_init_js() -> String {
    r#"(function initBadges() {
  window.updateBadgeStatus = function(status) {
    var states = { 'OFFLINE': '●', 'UP': '◉', 'RESIDENT': '◈' };
    var badge = states[status] || '○';
    document.querySelectorAll('[data-vixi-id*="daemon_status"]').forEach(el => {
      el.textContent = badge + ' ' + status;
    });
    ['baby_status', 'mama_status', 'papa_status'].forEach(id => {
      document.querySelectorAll('[data-vixi-id*="' + id + '"]').forEach(el => {
        el.textContent = badge;
      });
    });
  };
  var pollInterval = setInterval(function() {
    fetch('http://127.0.0.1:13013/status')
      .then(r => r.json())
      .then(d => { if (d.daemon_status) window.updateBadgeStatus(d.daemon_status); })
      .catch(() => {});
  }, 2000);
  window.updateBadgeStatus('OFFLINE');
})();"#.to_string()
}

fn generate_chat_js() -> String {
    r#"(function initChat() {
  window.triadAppend = function(text) {
    var log = document.querySelector('[data-vixi-id*="message_log"]');
    if (log) {
      var line = document.createElement('div');
      line.textContent = text;
      log.appendChild(line);
      log.scrollTop = log.scrollHeight;
    }
  };

  var input = document.querySelector('[data-vixi-id*="prompt_input"]');
  var send = document.querySelector('[data-vixi-id*="send_prompt"]');

  if (input && send) {
    send.onclick = function() {
      var text = input.value || '';
      if (text.trim()) {
        window.triadAppend('You: ' + text);
        window.postMessage({ type: 'infer', text: text }, '*');
        input.value = '';
      }
    };

    input.onkeypress = function(e) {
      if (e.key === 'Enter') {
        send.onclick();
        e.preventDefault();
      }
    };
  }
})();"#.to_string()
}

fn generate_pty_hearth_js() -> String {
    r#"(function initHearth() {
  var activeTab = 0;
  var tabNames = ['FORGE', 'BOOT', 'LIVE'];
  var tabBuffers = ['', '', ''];

  var hearth = document.querySelector('[data-vixi-id*="hearth"]');
  if (!hearth) return;

  var header = document.createElement('div');
  header.style.display = 'flex';
  header.style.gap = '8px';
  header.style.marginBottom = '8px';
  header.style.borderBottom = '1px solid #666';
  header.style.paddingBottom = '8px';

  var buttons = [];
  tabNames.forEach((name, idx) => {
    var btn = document.createElement('button');
    btn.textContent = name;
    btn.style.padding = '4px 8px';
    btn.style.cursor = 'pointer';
    btn.dataset.tabIdx = idx;
    btn.onclick = function() {
      activeTab = idx;
      updateHearth();
      buttons.forEach(b => b.style.fontWeight = b === btn ? 'bold' : 'normal');
    };
    if (idx === 0) btn.style.fontWeight = 'bold';
    buttons.push(btn);
    header.appendChild(btn);
  });
  hearth.appendChild(header);

  var pre = document.createElement('pre');
  pre.style.flex = '1';
  pre.style.overflow = 'auto';
  pre.style.fontSize = '10px';
  pre.style.fontFamily = 'monospace';
  pre.style.margin = '0';
  pre.style.padding = '8px';
  pre.style.backgroundColor = '#0A0705';
  pre.style.color = '#F7E9D2';
  pre.textContent = '$ [ConPTY shell tab ' + tabNames[activeTab] + '] (phase C stub)\n';
  tabBuffers[0] = pre.textContent;
  hearth.appendChild(pre);

  var input = document.createElement('input');
  input.type = 'text';
  input.style.width = '100%';
  input.style.padding = '4px';
  input.style.marginTop = '4px';
  input.style.fontFamily = 'monospace';
  input.placeholder = 'Type and press Enter to echo (phase C pty stub)';
  hearth.appendChild(input);

  input.onkeypress = function(e) {
    if (e.key === 'Enter') {
      var text = input.value;
      if (text) {
        tabBuffers[activeTab] += text + '\n';
        pre.textContent = tabBuffers[activeTab];
        pre.scrollTop = pre.scrollHeight;
        window.ipc && window.ipc.postMessage('pty:input:' + activeTab + ':' + text);
        input.value = '';
      }
    }
  };

  window.updateHearth = function() {
    pre.textContent = tabBuffers[activeTab];
    pre.scrollTop = pre.scrollHeight;
  };

  window.appendHearth = function(tabIdx, data) {
    if (tabIdx >= 0 && tabIdx < tabBuffers.length) {
      tabBuffers[tabIdx] += data;
      if (tabIdx === activeTab) {
        pre.textContent = tabBuffers[tabIdx];
        pre.scrollTop = pre.scrollHeight;
      }
    }
  };
})();"#.to_string()
}
