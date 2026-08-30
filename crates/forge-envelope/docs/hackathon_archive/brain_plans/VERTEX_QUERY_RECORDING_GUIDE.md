# Vertex AI Demo Recording Guide for Judges

This guide outlines three powerful, production-ready demo tracks you can record right now for the competition judges. 

---

## 🚀 Track 1: The Interactive Visual Studio UI (Highly Recommended)
*Perfect for presenting a polished, high-fidelity front-end showing how physical inspection metadata and live audio translate into dynamic shaderbind configurations and structured Vertex AI schemas.*

### 🛠️ How to Open and Run:
1. Open your browser (Chrome or Edge recommended).
2. Copy and paste the absolute file path into your URL bar:
   ```
   file:///F:/v3/crates/forge-envelope/surfaceledger/shaderbind_vertex_live.html
   ```
3. **Walkthrough / Recording Sequence:**
   * **The ADHD Harmonic Audio Pentatonic Dial**: Click the pentatonic note buttons (`C4`, `D4`, etc.) to play focus intervals. Watch the vibe bus bars (Omni RMS Energy, Transient Pulse, etc.) react in real-time, which feeds into the **SplitShader Vibe Surface WebGL canvas**.
   * **Hotswap VRAM Staging**: Click **"Flip VRAM Staging"** to showcase the double-buffered ping-pong hotswap (simulating the low-latency 17.89 ns hardware latency).
   * **Vertex AI Structured Audit**: Under *Google Cloud Vertex AI Test Pipeline*, toggle between the three built-in test scenarios:
     * **Walterdale Arch Defect**
     * **Moon Sentinel 252 Freeze**
     * **Nominal Treaty Sentry**
   * Watch the curvature slider dynamically adapt, and click **"Dispatch Vertex AI Audit"**. The system animates a real-time simulated query that renders the validated Pydantic JSON schema output and updates the rolling evidence SHA-256 chain links.

---

## 💻 Track 2: Real-time CLI Structured Query (Technical Proof)
*Perfect for proving that you are invoking live, zero-point deterministic models on Vertex AI, returning clean, validated JSON records matching your Rust/Pydantic schemas.*

### 🛠️ Execution Steps:
Execute the production-grade schema validation client script targeting the active `gemini-2.5-flash` model in your Vertex project:
```powershell
$env:GOOGLE_CLOUD_PROJECT="nde1-493505"
$env:GEMINI_MODEL="gemini-2.5-flash"
python F:\v3\crates\forge-envelope\scripts\vertex_schema_client.py
```
*This outputs the initial mock physical data (Walterdale Bridge steel arch delamination) and prints the fully-validated JSON record returned directly from the Google Cloud Vertex AI pipeline.*

---

## 📊 Track 3: Live Billing Credit Draw (Economics Proof)
*Perfect for demonstrating the real-world economics of your Vertex AI pipeline, verifying that your deep-context caching (75% discount) is drawing properly from your promotional/competition credits.*

### 🛠️ Execution Steps:
1. **Execute the GCP Billing credit validator**:
   Run the test harness which packages key repository files as a larger context payload (~10k-20k tokens) and sequential structured queries to verify actual pricing draw:
   ```powershell
   $env:GOOGLE_CLOUD_PROJECT="nde1-493505"
   python F:\v3\crates\forge-envelope\scripts\verify_billing_draw.py --model gemini-2.5-flash --queries 2 --no-confirm
   ```
   *This displays the exact token calculation (cached vs uncached input vs output) and projects the precise USD cost draw down to the fraction of a cent.*

2. **Verify Telemetry Logs**:
   Show the judges that the system logs every single transaction in real-time to persistent audit files:
   * [`F:\v3\crates\forge-envelope\surfaceledger\vertex_1hr_test_log.json`](file:///F:/v3/crates/forge-envelope/surfaceledger/vertex_1hr_test_log.json)
   * [`F:\v3\crates\forge-envelope\surfaceledger\billing_sentinel_status.json`](file:///F:/v3/crates/forge-envelope/surfaceledger/billing_sentinel_status.json)
