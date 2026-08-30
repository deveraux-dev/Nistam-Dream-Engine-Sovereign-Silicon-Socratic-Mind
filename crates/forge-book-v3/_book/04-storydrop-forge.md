# Storydrop · the 1000-PNG engine

`04 · authored synthesis · deveraux voice + ghostmoon law · sources: THE-2M-TOKEN-BOOK-REPORT.md (2M-token verified technique report, 2026-07-10) · author-dossier.md §V (haiku pool) · DeadMetalPoets.html (style seed) · code: tools/storydrop-forge/{rules,haiku,styles,state,storydrop}.py · skill: .claude/skills/storydrop-forge/SKILL.md · prove: python tools/storydrop-forge/storydrop.py run <beats.txt> --slug <s>`

## Prose

Seventy agents spent two million tokens to learn eight things about a picture held on screen too long or too short. The report is honest about the bill: no model pin, an unbounded verification multiplier, three fixes now living in the script that spent it. Everything that survived the adversarial pass is a rule about time — how long an image must sit before it is remembered, how close two important pictures can land before the second one goes unseen, what a silence between panels is for. Sean's own read on it: pin the model before you shape the wave.

The engine takes prose, splits it into beats, and stamps a role on each one — establisher, initial, peak, release — the same shape a Blackfoot or Cheyenne story arc is built on, the same shape Cohn found in the brain's own parsing of a wordless comic. Ki, shō, ten, ketsu — introduce, develop, twist, recontextualize. No conflict required. The twist does not have to fight anything. It has to change what the first two acts meant.

The twist does not resolve conflict; it resolves perspective. In the context of NARRATIVE-BIBLE Section III's reclamation arc—erasure, distance, clarity—the twist is the moment witness becomes possible. The first two acts establish what was hidden or distorted; the twist steps back far enough to see it whole. This is the substrate of the engine itself: built not from institutional support or inherited access, but from lived material constraints (NARRATIVE-BIBLE Section V), where every frame and silence carries the weight of recovery. The recontextualization in ketsu is not aesthetics; it is the act of claiming—the testimony that what was erased is now seen, and the act of seeing is proof that the person who sees still exists.

Every time the story changes scene, one frame goes quiet. No characters, no plot, held several seconds — a pillow shot, the technique Burch found in Ozu. Into that quiet the engine drops one of eight haiku pulled from the dossier: the seven gates, the rakugo punchline, Njal saying nothing, the copy made three times to survive the fall, the knock that already knows what the cave means, the thousand and one frames that never close, the tiger inside the tiger, the moral that walks out and finds the reader. Eight silences, eight ways of not-saying, cycling in order so the reel never repeats one twice in a row.

The palette rotates too — never the same look back to back, six built on the same rust-and-noise canvas language the desktop already speaks in. And before any of it renders, the beat text runs through the same linter that grades the rest of this book: imagery over abstraction, the short sentence over the long one, land on a period. A story that fails that gate still renders. It just carries the receipt saying where it was thin.

## Terse Code

```
ENGINE=storydrop;IN=beats.txt(blank-line-sep)->OUT=frames/*.png+manifest.json+state.json
SECTION=kishotenketsu{ki:.30,sho:.30,ten:.30,ketsu:.10}
ARC=cohn{size=4;roles=[establisher,initial,peak,release];PEAK_REQ=1;PEAK!=first;INITIAL!=last}
PATTERN=fixed{3|4|5};resolve=last_iteration_only
DWELL{subliminal=13ms;motion=100ms;plot_min=300ms;plot_safe=500ms;pillow=4x_safe}
BLINK_DEAD=[200,500]ms->chain_or_pad(fix=501ms)
TRANSITION=mccloud6{action:.65/.20/.15=a2a/s2s/scn2scn;contemplative+=m2m,aspect2aspect}
PILLOW=on(section_change);content=haiku_only;plot_text=0
HAIKU=dossier_sec_V{n=8;tags=[descent,rakugo,saga,samizdat,miners,scheherazade,panchatantra,openframe]};rotate=cycle,no_repeat_immediate
STYLE=6_palette{ember_rust,void_cyan,ash_bone,signal_green,copper_blood,glacier};base=DeadMetalPoets{noise+scanline+artifact};pick=exclude_last(state.style_history)
LINT=forge-dialogue/voice_lint.py--mode=story;GATE=advisory(4.0+=ship,report_names_thin_beats);BLOCK=0
KILLED=[katabasis_always_returns,fastcut_harms_EF,scan_shift_habituation,rsvp_50pct_any_rate,active_dda_beats_telemetry,ascending_peaks_curve,visual_first_EF,reward_immediacy_delay_aversion,explicit_inference_bridge,swink_triad(6not3)];DO_NOT_REENCODE=1
COST_LAW=(2M-token postmortem)pin_worker_model=sonnet;cap_numeric_claim_pool=20;budget_guard_pre_launch=1;multiplier_stage=where_cost_lives_not_fanout_count
```

## Runbook

```powershell
cd F:\NewRepo\tools\storydrop-forge

# one story, blank-line beats, pattern number 4, style auto-rotates
python storydrop.py run F:\path\to\beats.txt --slug my-story --pattern 4

# check phase completion
python storydrop.py status my-story

# outputs
#   F:\NewRepo\tools\storydrop-projects\my-story\frames\f0001.png ...
#   F:\NewRepo\tools\storydrop-projects\my-story\manifest.json      (per-frame section/role/dwell/style/haiku)
#   F:\NewRepo\tools\storydrop-projects\my-story\voice-lint-report.md
#   F:\NewRepo\tools\storydrop-projects\my-story\state.json         (style_history — rotation memory)

# re-run same slug = new style (excludes prior), same haiku cycle order, fresh render
python storydrop.py run F:\path\to\beats.txt --slug my-story --pattern 4
```

Gate before any drop ships: `voice-lint-report.md` average >= 4.0, zero em-dash/poison/corp violations in live prose, every pillow shot carries a haiku and nothing else, no killed rule from the 2M-token postmortem reintroduced.

## The Reel — the same engine, turned on a folder that already holds pictures

`04b · authored synthesis · deveraux voice · code: tools/storydrop-forge/reel_forge.py (sibling to storydrop.py; reuses rules/haiku/styles) · drop: tools/storydrop-projects/ironroot-entrance/ · plan: _plans/PLAN-ironroot-reel.md · prove: python tools/storydrop-forge/reel_forge.py <folder> --extra-dir <concept-dir> --max-beats 58`

### Prose

Storydrop renders prose into frames. Its sibling runs the loop backward: it takes a folder already full of pictures and finds the story hiding in the order of them. Point it at two weeks of dev screenshots and sixteen faction dossiers and it reads the chronology as a descent — the build as a katabasis — and stitches the finished worlds in as the reveal. The dossier paintings become pillow shots, the world at rest between the frantic making. The gate-descent frames lead, the way a story opens mid-fall.

Over the pictures it lays two voices. The eight haiku ride the pillows, as they always have. And on the montage it writes pieces of Sean's own story — the datasheet at thirty-nine, the father's air growing heavier, the integers that do not drift, the wendigo that comes if you take too much, the Ghost Moon present only by being named absent. Each line arrives a held interval after the last, slow enough to read and no slower, matted so the whole reel lands inside two and a half minutes.

The sound is not music. A sub-bass drone builds from silence. A heartbeat sits where a backbeat would. An air-raid siren rises on the turn, three knocks land on the miners' poem, and then the drums cut and the last frame holds in the dark. It closes on the open frame — the moral walks out and finds the reader. All of it is built on the DeadMetalPoets calcification, so the reel reads like a 1998 memory dump that learned to grieve. The same MIDI it plays on channel ten, it also emits — so a recorded voice, or a studio, can lock to the drum track and match the words.

### Terse Code

```
ENGINE=reel_forge (sibling storydrop; reuses rules/haiku/styles)
IN=image_folder + N*--extra-dir(concept art, curated >250KB, dedup) -> OUT=reel.html + cue-sheet.{json,md} + narration-script.md + reel_fx.wgsl
CLASSIFY: Screenshot*=plot(chrono) · f####=gate-descent(lead) · dossiers=pillow · twit/kr=drop
STORY=River-and-the-Suit{ki:datasheet/armor · sho:father/integers · ten:13moons/wendigo/ghostmoon · ketsu:river/landing}; on non-pillow beats per act
DWELL: haiku-pillow=word_gap*lines+800 · story=1500+46*len · montage=dmin · point=dmax; band MATTED to 2:00-2:30
SOUND=WebAudio+MIDI ch10{drone 52+78Hz · heartbeat thud41 · ghost snare38 · knock37 · siren(ten) · build-by-act · silent close}
VISUAL=DeadMetalPoets{memdump/SIGSEGV cold-open · rust border · dead-silicon grain · cold bloom · scanline · glitch-tear(ten) · flicker(words) · chainlink(ten) · push-in reveals}
MIDI2.0=note-on steps · emits note+drums · sub-ms timestamp sync = record-and-match spine
reel_fx.wgsl=forge-gpu void_compression-convention post SEED(beat_pulse) — dormant, RENDERED=readback-owed (Sean-gated)
```

### Runbook

```powershell
cd F:\NewRepo\tools\storydrop-forge
python reel_forge.py "C:\Users\seanm\Desktop\1000" `
  --extra-dir "E:\.airgap\2026-05-17-dsp-hrtf-p00-loop\ironroot-edict\game\assets\concept\concepts-v3" `
  --slug ironroot-entrance --max-beats 58 --max-pillows 9 --palette ember_rust
# open reel.html, click to arm audio, screen-record. tune runtime: --max-beats / --max-pillows / --word-gap-ms (hold 2:00-2:30)
```

