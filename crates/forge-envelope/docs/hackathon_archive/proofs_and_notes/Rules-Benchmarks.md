# All Things Agentic Hackathon — Benchmark Ledger

Source: `C:\Users\seanm\Desktop\Rules.txt` (Devpost Official Rules, Google LLC sponsor / Devpost Inc. administrator).
Lossless recompression: every operative requirement below traces to a line in the source. Chrome/nav/footer text dropped (non-operative).

Scoring shorthand: **HIT** = benchmark met. **MISS** = disqualification or point loss.

---

## 0. CLOCK (today = 2026-08-20)

| ID | Gate | Deadline (PT) | Slack from today |
|----|------|---------------|------------------|
| D1 | Google Cloud $150 credit request form (`forms.gle/riGhgDSHkHeMx8Ca6`) — 1 code/entrant, reviewed ≤72 business hrs, not guaranteed | Aug 28, 12:00 PM | **8 days** |
| D2 | **Submission closes** — all materials final | **Aug 31, 5:00 PM** | **11 days** |
| D3 | Judging window (no entrant action; project must stay live & free) | Sep 1 9:00 AM → Oct 1 11:45 PM | — |
| D4 | Winners announced | ~Oct 8, 10:00 AM | — |
| D5 | If notified as winner: **respond ≤2 days** from first attempt or forfeit | post-D4 | — |
| D6 | Return Declaration of Eligibility / Liability / Publicity Release ≤2 days after notification | post-D4 | — |
| D7 | Return all Required Forms (W-9 US / W-8BEN non-US) ≤10 business days | post-D4 | — |
| D8 | Prize delivered ≤60 days after Devpost receives Required Forms | post-D7 | — |

Contest period opened Aug 3, 2026 9:00 AM PT. Entrant is responsible for their own timezone math.

---

## 1. HARD GATES — any MISS = ineligible (Stage One pass/fail)

| ID | Benchmark | Pass condition |
|----|-----------|----------------|
| G1 | Age | At or above age of majority in your jurisdiction (≥20 in Taiwan) at time of entry |
| G2 | **Residency** | NOT resident of: Italy, **Quebec**, Crimea, Cuba, Iran, Syria, North Korea, Sudan, Belarus, Russia, or any OFAC-designated country. Void where prohibited. |
| G3 | Sanctions | Not a person/entity under US export controls or sanctions; not ordinarily resident in an embargoed country |
| G4 | Connectivity | Internet access as of Aug 3, 2026 |
| G5 | Conflict of interest | Not an employee/intern/contractor/office-holder of Google, Devpost, or any org involved in Contest design/production/promotion/execution/distribution — nor their immediate family (parents, siblings, children, spouses, life partners) or household members. Not a government-agency employee. Sponsor adjudicates conflicts at sole discretion. |
| G6 | Employer authorization | If entering via employer/team: employer has full knowledge + consent, including your potential prize receipt; entry does not violate employer policy |
| G7 | Devpost account | Registered on `allthingsagentichackathon.devpost.com` |
| G8 | **Newly created** | Project built **during** Aug 3–31 submission window. Frameworks, libraries, starter templates, AI coding assistants allowed. **Any other pre-existing code/work must be disclosed.** |
| G9 | Original & sole ownership | Your original work; solely owned; no third party holds any right or interest; violates no copyright/trademark/patent/contract/privacy right. Third-party technical assistance allowed only if output remains solely your work product + your ideas. |
| G10 | Open source | If OSS used: comply with its licenses **and** your software must enhance/build upon the OSS's features — not merely repackage it |
| G11 | Third-party SDK/API/data | You are authorized under each tool's T&Cs / licensing |
| G12 | No Sponsor support | Project not developed with (or derived from a project developed with) financial or preferential support from Google/Devpost — no funding, investment, contract work, or commercial license from them before submission close |
| G13 | English | App supports English at minimum; all submission materials in English or with English translation (video, description, testing instructions, everything) |
| G14 | Content clean | No derogatory / offensive / threatening / defamatory / disparaging / libelous / inappropriate / indecent / sexual / profane / tortuous / slanderous / discriminatory content; nothing promoting hatred or harm |
| G15 | Lawful | No unlawful content under federal/state/local law of the country where you created it **and** of the United States |
| G16 | No third-party branding | No third-party advertising, slogan, logo, or trademark implying sponsorship/endorsement |
| G17 | No rights violations | No content violating a third party's publicity, privacy, or IP rights |
| G18 | Reasonably addresses a challenge | Stage One explicitly checks: all requirements present + reasonably addresses a challenge + reasonably applies the requirements |
| G19 | Fair play | No cheating, deception, unfair practices, tampering, or harassment of entrants/Google/Judges |
| G20 | Truthful info | No false identity, address, phone, email, or ownership claims — immediate elimination |

**Multiple submissions:** allowed, but each must be *unique and substantially different*, at Sponsor/Devpost sole discretion. **Each project wins at most one (1) prize.**

---

## 2. MANDATORY STACK — all three required, every category

| ID | Benchmark | Pass condition |
|----|-----------|----------------|
| S1 | Model | **Gemini 3.5 or newer**, accessed via **Gemini API or Vertex AI** |
| S2 | Agent framework | **≥1 of:** Google ADK, GenAI SDK, Antigravity SDK, GenKit |
| S3 | Cloud infra | **≥1 Google Cloud infrastructure service** (Cloud Run, Cloud SQL, Firestore, GKE, Pub/Sub, …) |
| S4 | Mandate | Autonomous agent that **operates beyond standard chat loops**: runs asynchronously in the background, handles heavy lifting of complex workflows, or dynamically manipulates data pipelines/representations. Built **and deployed**. |
| S5 | Billing | You pay all Google Cloud fees exceeding the $150 credit |

---

## 3. CATEGORY — select exactly one (Sponsor may reassign)

| ID | Category | Build benchmark |
|----|----------|-----------------|
| C1 | **Taskmaster** | Agent takes *action*, not text. Target a messy multi-step chore from your job/classes/personal life. Agent handles details, routes the right info to the right places, proves it does the heavy lifting. |
| C2 | **Collaborative Partner** | Agent leads and takes notes: asks clarifying questions, guides step-by-step, has a clear feedback-capture mechanism, continuously adapts to the user's way of thinking. |
| C3 | **Fortified Enterprise Fleet** | Scalable network of institutional agents on official enterprise infra. Must demonstrate: (a) agents cataloged for cross-department use, (b) safe context maintenance across **weeks** of async operation, (c) interaction with production data without violating enterprise compliance, data sovereignty, or security policy. |

C3 recommended tech (Gemini Enterprise Agent Platform), by axis:
- **Discovery & Lifecycle** — Agent Registry (publish / version / discover approved agents)
- **Core Execution & State** — Agent Runtime (long-running async background execution) + Memory Bank (persistent secure cross-session context over extended timelines)
- **Security & Governance** — Agent Identity (zero-trust access control), Agent Gateway (unified routing + policy enforcement), Model Armor (inline guardrails vs. prompt injection, tool poisoning, PII leaks)
- **Telemetry** — Agent Observability (OpenTelemetry-compliant audit logs + end-to-end reasoning-chain traces)

---

## 4. DELIVERABLE MANIFEST — every artifact, all required unless marked

| ID | Artifact | Pass condition |
|----|----------|----------------|
| A1 | Devpost submission form | All required fields complete, submitted before D2 |
| A2 | Category selection | Exactly one |
| A3 | Hosted project URL | Web UI / Chrome extension / mobile app / functioning demo / test build. Technically "if available" — **"highly encouraged"; treat as required.** |
| A4 | Free + unrestricted test access | Working project usable free of charge, no restrictions, by Sponsor/Admin/Judges **until Judging Period ends (Oct 1)**. If private: **login credentials in testing instructions.** |
| A5 | Text description | Features + functionality summary, technologies used, other data sources used, **findings and learnings** from building it |
| A6 | Code repo URL | GitHub, GitLab, or Bitbucket. If private: **grant access to `testing@devpost.com` AND `cloudhackathons@google.com`** |
| A7 | Spin-up instructions | Step-by-step in `README.md`: set up + run locally, or deploy to cloud. Proves reproducibility even if judges never run it. |
| A8 | Architecture diagram | Clear visual of the system — how Gemini connects to backend, database, frontend. Must live in the **public GitHub repo** (per judging criterion 3). |
| A9 | Demo video | ≤**4:00** (only first 4 min evaluated). Public (not unlisted) on **YouTube or Vimeo**, link on submission form. English or English subtitles. |
| A10 | Video content | (a) short overview of the problem solved, (b) value proposition, (c) demo of app in action, (d) **proof backend runs on Google Cloud** — Cloud Console, Cloud Run dashboard, Vertex AI logs, or a `.run` URL on screen |
| A11 | Pre-existing work disclosure | Any incorporated pre-existing code/work declared (see G8) |
| A12 | Team roster | If team: all members added as Project members on Devpost; one appointed **Representative** authorized to act/submit on the team's behalf |
| A13 | Hardware (conditional) | If the project runs on proprietary/non-public hardware (anything beyond smartphone/tablet/desktop), Sponsor may demand physical access to it |

---

## 5. SCORING — Stage Two, criterion scored 1–5, weighted, averaged

### 5A. Innovation & Operational Utility — **40%**
Benchmark: *does the system eliminate real-world friction?* Judges want high-value **autonomous execution**, not chat queries. **The "Twist" must be present.**

- **Taskmaster** — Agent **intercepts and completes a multi-step background workflow with zero human intervention**. Team exercises the **"Bring Your Own Friction" (BYOF)** mandate on a unique, *personal* problem.
- **Collaborative Partner** — Agent **actively synthesizes or mutates** data, not just reads it. Team ingests **unusual, messy, or highly complex unstructured** data streams.
- **Fortified Enterprise Fleet** — Task complex enough to *warrant* multi-agent. System **intelligently delegates to specialized sub-agents**. Built for an **"Unlikely Hero"** outside standard corporate roles.

### 5B. Architectural Discipline & Tech Stack — **30%**
Benchmark: *engineering decisions, not API-calling ability.* Decoupling, state management, robust failure-tolerant agentic design.

- **The Continuous Action Engine** — clean, modularized, maintainable system; explicit state-management story; **tools properly isolated and scoped for security**.
- **The Evolving Knowledge Engine** — data architecture: intelligent schema design, efficient vector embedding strategy, efficient management of **massive context windows**.
- **The Multi-Agent Nexus** — good agent workflows; **clear, strictly enforced separation of concerns** between agents; **failure-tolerant inter-agent routing** — you must show recovery when a worker agent loops or returns a hallucination.

> [INFERRED] These three engine names do not appear elsewhere in the Rules; they map 1:1 in order to Taskmaster / Collaborative Partner / Fortified Enterprise Fleet. Not stated in source — safest play is to satisfy the bullet matching your category **and** name it that way in the README so the judge's checklist matches your text.

### 5C. Demo & Production Readiness — **30%**
Benchmark: *clarity of technical documentation + undeniable proof of execution.* The 4-minute video must clearly define the friction being solved **and** explain the architecture.

- **Proof of Action** — video shows **unedited, live execution** of the agent performing its task, visible via terminal logs, database updates, or UI changes.
- **Documentation** — public GitHub repo has a **clean architecture diagram** and **reproducible setup instructions**; **visual proof of Google Cloud deployment in the video**.

---

## 6. BONUS — Stage Three, max **+1.0**, final score ceiling **6**

| ID | Bonus | Points | Pass condition |
|----|-------|--------|----------------|
| B1 | Published build content | +0.2 max | Blog / podcast / video on any public platform (medium.com, dev.to, YouTube, …) covering how the project was built. Must be **public, not unlisted**. Must include language stating you created it **for the purposes of entering this hackathon**. |
| B2 | Social post | +0.2 max | Post on X, LinkedIn, Instagram, or Facebook promoting the project. On X/LinkedIn include hashtag **#AllThingsAgenticHackathon**. |
| B3 | Extra Google AI models | +0.2 each, **+0.6 cap** | Successfully integrate additional Google AI models — Gemma, Veo, Lyria, etc. → **3 models = full 0.6** |

Base score 1–5 (weighted average of 5A/5B/5C) + up to 1.0 bonus = **Final 1–6**.

---

## 7. PRIZE TABLE — target selection

| Prize | Qty | Cash | GCP credits | Extras | Eligible |
|-------|-----|------|-------------|--------|----------|
| **Grand Prize** | 1 | $50,000 | $5,000 | Virtual coffee w/ Googler + social promo | Highest score across all categories |
| The Taskmaster | 1 | $20,000 | $2,000 | same | Highest in that category |
| The Collaborative Partner | 1 | $20,000 | $2,000 | same | Highest in that category |
| The Fortified Enterprise Fleet | 1 | $20,000 | $2,000 | same | Highest in that category |
| Startup Excellence | 1 | $20,000 | $5,000 | same | **Must submit on behalf of an incorporated organization + provide corporate email** |
| Individual/Hobbyist (Best Team/Solo Build) | **2** | $10,000 | $1,000 | same | All eligible individuals/teams |
| Best Architectural Design | **2** | $5,000 | $1,000 | — | Top scorers on that criterion (5B) |
| Best Multimodal UX | **2** | $5,000 | $1,000 | — | Top scorers on that criterion |
| Honorable Mentions | **5** | $2,000 | $500 | — | Runners-up |

Tiebreak: compare criterion scores **in listed order** (Innovation → Architecture → Demo); if still tied, judges vote. Disqualified winner → next-highest score promotes. No prize awarded if a region receives zero entries. Judges' determinations final and binding. Judging may run multiple rounds/panels and may use expert panels, peer review, **automated AI-driven analysis**, or any combination. Judges may score on description/images/video alone — they are **not required to run your project**.

> [INFERRED] "Best Multimodal UX" has no matching Stage Two criterion in the source; its "top scoring projects in that judging criteria" clause is unanchored. Multimodal I/O is therefore free upside with no defined rubric — build it, don't bank on it.

---

## 8. STANDING OBLIGATIONS (non-scoring, binding on entry)

- **Binding agreement** — submitting = accepting the Rules; a legal agreement between you and Google.
- **License grant to Google** — perpetual, irrevocable, worldwide, royalty-free, non-exclusive license to use, reproduce, adapt, modify, publish, distribute, publicly perform, derive from, and publicly display your Project: (1) for judging, (2) for advertising/promotion, including screenshots, animations, video clips. **You retain ownership and all IP/moral rights.** Commercially available third-party software you don't own (and Google can procure without undue expense) is excluded from the grant.
- **Publicity** — you consent to promotion/display of your Submission and use of your name, likeness, photograph, voice, opinions, comments, hometown, country — any media, worldwide, no further payment or right of review, unless prohibited by law.
- **Privacy** — Google may collect/store/share/use PII (name, mailing address, phone, email) per `policies.google.com/privacy`; data may transfer outside your country including to the US, which may have weaker privacy law. Withholding mandatory registration data = right to disqualify. Access/review/rectification/deletion requests → `cloudhackathons@google.com`. Devpost's own privacy policy also applies (`info.devpost.com/privacy`).
- **Fees & taxes** — you (and every participating team member) pay wiring fees, currency-exchange fees, and all federal/state/provincial/local taxes; may need W-9 (US) or W-8BEN (non-US); comply with your own FX and banking-reporting rules. Sponsor/Devpost may withhold part of the prize for tax compliance.
- **Prize delivery** — cash goes to the individual, the team's Representative, or the organization; the Representative allocates among members. Non-cash prizes are not redeemable for cash; ARV may be adjusted by jurisdiction; Sponsor may substitute a prize of equal or greater value. No warranties of any kind on prizes.
- **Indemnity & release** — you indemnify Google/Devpost/all Contest Entities against claims from your acts, IP infringement, misrepresentation, non-compliance, third-party claims, and prize use. You release Google from liability for Contest Site malfunctions, entry-processing errors, and typographical errors in prizes/winners.
- **No employment** — nothing here creates an offer, contract, confidential, fiduciary, or agency relationship. Submitted voluntarily, not in confidence or trust.
- **Sponsor's kill switch** — Google may cancel, terminate, modify, or suspend the Contest for virus, bugs, tampering, fraud, or technical failure; may disqualify tamperers and seek damages.
- **Internet risk is yours** — Contest Entities not responsible for lost/late/garbled/undeliverable submissions from any system, network, hardware, software, or congestion failure. **Corollary benchmark: submit early, not at 4:59 PM on Aug 31.**
- **Draft policy** — you may save drafts to your portfolio before submitting; after D2 **no changes to the Submission** (portfolio project may keep updating). Post-deadline edits allowed only if Sponsor/Devpost permits, and only to remove infringing marks, PII, or inappropriate material — the Submission must remain substantively the same.
- **Governing law** — California law, conflict-of-law rules excluded. Litigation/injunctive relief rights waived to the extent permitted. **Binding arbitration** via JAMS in the San Jose, CA area, one mutually agreed arbitrator, costs split equally. Severability applies.
- **Devpost ToS** (`info.devpost.com/terms`) incorporated by reference; "Poster" = "Sponsor"; these Official Rules control on conflict.
- Support: `support@devpost.com`.

---

## 9. CRITICAL-PATH BENCHMARK SEQUENCE (11 days)

1. **Verify G2 residency** before spending another hour — Quebec is an excluded jurisdiction.
2. **Fire D1 credit form today** — 72 *business* hours review + Aug 28 cutoff means the real deadline is ~Aug 22 to be safe.
3. **Lock C1/C2/C3 category** — it selects which 5A and 5B bullet you're graded against.
4. **Stand up S1+S2+S3 end-to-end on Google Cloud first**, before feature work — a project missing any one of the three fails Stage One regardless of quality.
5. **Build the autonomy proof (5A)** — the single highest-weighted thing is an agent completing a multi-step workflow with no human in the loop.
6. **Instrument for the video (5C)** — terminal logs / DB rows / UI changes that visibly move during a live unedited run. Design this in; you cannot retrofit it in the last 24 hours.
7. **README + architecture diagram in the public repo (A7, A8)** — counts twice: manifest requirement and 5C scoring.
8. **Record ≤4:00 video** hitting: friction → value prop → architecture explanation → live unedited run → Google Cloud console on screen.
9. **Bank the +1.0 bonus (B1, B2, B3)** — cheapest points in the contest. Blog post + tagged social post + 3 extra Google models.
10. **Submit ≥24h before D2.**

---

## 10. SCORE MATH — the numbers you are actually chasing

Let **I** = Innovation (1–5), **A** = Architecture (1–5), **D** = Demo (1–5).

```
Base   B = 0.40·I + 0.30·A + 0.30·D          B ∈ [1.00, 5.00]
Final  F = B + bonus                          bonus ∈ [0.00, 1.00],  F ∈ [1.00, 6.00]
```

### Marginal value of one point

| Move | Δ Final | Effort |
|------|---------|--------|
| +1.00 on Innovation | **+0.40** | Hardest — this is the whole product |
| +1.00 on Architecture | +0.30 | Weeks of engineering |
| +1.00 on Demo | +0.30 | Days of instrumentation + editing |
| B2 social post w/ hashtag | **+0.20** | **~5 minutes** |
| B1 blog/video on the build | **+0.20** | ~2 hours |
| B3 three extra Google models | **+0.60** | ~hours of integration |

### The three facts that decide this

**F1 — Full bonus is worth exactly as much as being the best project in the contest.**
Straight 4s → B = 4.00. Straight 5s → B = 5.00. Delta = **+1.00**.
Full bonus = **+1.00**. Same number. One is a blog post, a tweet, and three API integrations. The other is beating 6,296 people on every axis.

**F2 — A perfect project with no bonus loses to a merely-good one with full bonus.**
```
straight 5s, zero bonus     → 5.00
4.2 average, full bonus     → 5.20   ← wins
```
Base caps at 5.00. Bonus does not stack onto a cap — it stacks *past* it. **Skipping the bonus mathematically removes you from the top of the range.** This is non-negotiable, not optimization.

**F3 — Innovation is double-counted.** 40% weight **and** the first tiebreak (ties resolve Innovation → Architecture → Demo, then judge vote). At equal totals, the autonomy proof wins.

### Targets to hit

| Band | Base needed | With full bonus | [ASSUMED] outcome |
|------|-------------|-----------------|-------------------|
| Stage One | — | — | Pass/fail only. No score. |
| Honorable Mention | ~3.5 | 4.5 | Competent + complete |
| Category win | ~4.3 | **5.3** | Straight 4.5s + full bonus |
| Grand Prize | ~4.7 | **5.7** | Near-straight 5s + full bonus |

The band values are [ASSUMED] — the Rules publish no score thresholds, only "highest-scoring wins." The *math* above (F1–F3) is exact and derived from the stated weights.

**Practical target: 4.5 / 4.5 / 4.5 + 1.00 bonus = 5.35 final.** That is a winnable number in 11 days. Straight 5s is not.

---

## 11. FIELD & SURFACE AREA

- **6,296 participants** registered (observed, source line 19). Participants ≠ submissions.
- **16 prizes total**: Grand 1 · Taskmaster 1 · Collaborative 1 · Fortified 1 · Startup 1 · Individual/Hobbyist 2 · Best Architectural 2 · Best Multimodal UX 2 · Honorable Mentions 5.
- **7 of 16 are not category-capped** (2 Individual/Hobbyist + 5 Honorable Mentions) — you compete for those from any category.
- **2 more are criterion-only** (Best Architectural Design, Best Multimodal UX) — these reward a single axis, not the whole package. A project that is architecturally exceptional but demos poorly still has a $5,000 + $1,000 lane.
- Odds explicitly depend on entries received **and entrant skill** (§9). No prize awarded to a region with zero entries.

**Surface-area levers available under the Rules:**
1. **Multiple submissions** are permitted if *substantially different* (Sponsor's sole discretion). Each project caps at one prize. Two projects = two shots at 16 slots. [At 11 days out, this likely splits your effort below the 4.3 bar — noted as available, not recommended.]
2. **Startup Excellence** — **LIVE.** Entity: **2748684 Alberta LTD** (incorporated ✓). $20,000 + **$5,000** credits, the largest credit award outside Grand Prize. See §13.
3. **Best Multimodal UX** has no defined rubric in the Rules (§7 note). Undefined criterion + 2 slots = the softest target on the board. See §14.

---

## 12. FLAG — G8 collision

**G8 requires the project be newly created during Aug 3–31, 2026.** Pre-existing code may be incorporated but **must be disclosed**, and "the work described and submitted must have been built during the Submission Period."

`F:\v3` predates the window. If any part of the forge is going into this, the compliant shape is: the *agent system* is new work built in-window, and the forge is disclosed as an incorporated pre-existing dependency — the same way a framework or library is. What cannot happen is submitting existing v3 work as the project itself.

Not a blocker. It is a disclosure requirement and a scoping decision, and it needs to be settled before step 3 of §9, because it determines what you're actually building.

---

## 13. STARTUP EXCELLENCE — 2748684 Alberta LTD

Prize: **$20,000 USD + $5,000 GCP credits + virtual coffee + social promo.** Qty 1.

Source requirement (line 352), verbatim: *"To be eligible for the StartUp Prize, you must be submitting on behalf of an organization which must be incorporated and you must provide your corporate email address."*

Two conditions. Exactly two.

| # | Condition | Status |
|---|-----------|--------|
| 1 | Submitting on behalf of an organization, incorporated | ✓ **2748684 Alberta LTD** |
| 2 | Provide **corporate email address** | ✗ **`morin.sean123@gmail.com` will not satisfy this** |

**The only gate is an email on a company domain.** Register a domain, point a mailbox at it, use it on the Devpost submission. ~1 hour, ~$15/yr. Do it before D2, ideally before you register the Devpost submission so the address is consistent across every artifact.

**Value:** this is a fifth eligibility lane on the *same* project, and structurally the thinnest field on the board — most hackathon entrants are individuals, not incorporated entities. Per §9 each project still wins at most one prize, so this buys *chances*, not stacked payouts. Four lanes beats three.

**Downstream consequences of entering as the organization:**
- **G6 consent** — trivially satisfied, you own the entity.
- **Representative** — you must be appointed and authorized to act and submit on the org's behalf (line 114). Self-appointment is fine; just be consistent on the form.
- **Prize delivery** — cash goes to the organization's bank account, not to you personally (§9C). Representative allocates internally.
- **Tax forms (D7)** — the Rules name W-8BEN for non-US residents. [INFERRED] An entity files **W-8BEN-E**, not W-8BEN. Expect the corporate variant if the org is the payee; don't let a wrong-form round-trip eat the 10-business-day window.
- **G2 residency** — the entity being Alberta is not proof of *your* residence, and G2 tests where **you** reside. Alberta is not Quebec, so this reads clear — confirm it yourself, it is the one gate that voids everything else.

---

## 14. BEST MULTIMODAL UX — the dangling criterion

Prize: **$5,000 USD + $1,000 GCP credits.** Qty **2**.

**Observed:** the string "Multimodal" occurs **exactly once** in the entire Rules document — line 384, the prize table row. Verified by full-document search for `multimodal|modal|UX|user experience|interface|voice|image|video|audio`. No definition, no criterion, no rubric anywhere.

Eligibility reads *"Top scoring projects in that judging criteria."* **That criterion is not published.** Stage Two defines three and only three: Innovation & Operational Utility (40%), Architectural Discipline & Tech Stack (30%), Demo & Production Readiness (30%).

The row directly above it — Best Architectural Design — uses the **identical** eligibility phrasing and *does* anchor cleanly to criterion 2. One template line anchored, one dangling.

> [INFERRED] A Stage Two rubric item covering multimodal/UX was cut and the prize table was not updated. Common in copy-forwarded hackathon rules. Not stated in source.

**Why it is the softest target on the board:**
1. Two prizes still get awarded — the obligation doesn't vanish with the rubric.
2. No published bar means no competitor can be measured as clearly beating you.
3. Most entrants optimize against the three *published* criteria. This one is invisible unless you read the prize table against the rubric.

**Why it costs you nothing extra — the B3 overlap:**

| Model | Modality | B3 bonus | Multimodal UX surface |
|-------|----------|----------|----------------------|
| Gemini 3.5 | Native multimodal **input** (vision, audio) | — (mandatory S1) | **Free** — already required |
| Veo | Video **generation** | +0.2 | ✓ |
| Lyria | Music **generation** | +0.2 | ✓ |
| Gemma | Text | +0.2 | ✗ (bonus only) |

Veo + Lyria = **+0.4 bonus** *and* the exact output-side modality spread this prize rewards. Add Gemma to cap B3 at **+0.6**. One integration pass, two payouts.

**Benchmark:** ship a UX with ≥3 modalities crossing input and output — e.g. voice or image **in** (Gemini native), generated video **out** (Veo), generated audio **out** (Lyria) — and put all three on screen in the 4-minute video. That single build simultaneously banks +0.6 of bonus (§6 B3), feeds criterion 5C's proof-of-action, and is the strongest available claim on an unrubriced $5,000 + $1,000 prize with two open slots.
