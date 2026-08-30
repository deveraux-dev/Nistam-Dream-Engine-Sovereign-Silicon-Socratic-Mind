// Forge V3 Demo Shell — the 5D sky IS the window; one glass terminal rides it.
// ONE star clock: one 5D fetch (get_starmap_5d), one skyFrame rAF repaint.
(function () {
  'use strict';

  const $ = (id) => document.getElementById(id);

  function getTauri() { return window.__TAURI__ || null; }
  async function invokeCommand(cmd, args = {}) {
    const t = getTauri();
    try {
      if (t && t.core && t.core.invoke) return await t.core.invoke(cmd, args);
      if (t && t.invoke) return await t.invoke(cmd, args);
    } catch (e) {
      console.error(`[shell] ${cmd} refused:`, e);
      return null;
    }
    console.warn(`[shell] Tauri invoke unavailable for '${cmd}'.`);
    return null;
  }

  async function listenEvent(event, handler) {
    const t = getTauri();
    try {
      if (t && t.event && typeof t.event.listen === 'function') {
        return await t.event.listen(event, handler);
      }
      if (t && typeof t.listen === 'function') {
        return await t.listen(event, handler);
      }
      if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.listen === 'function') {
        return await window.__TAURI_INTERNALS__.listen(event, handler);
      }
    } catch (e) {
      console.warn(`[shell] listenEvent '${event}' error:`, e);
    }
    return () => {};
  }

  const so5State = {
    theta_zw: 0.0,
    phi_wv: 0.0,
    beta_lorentz: 0.0,
    autoSpin: false,
    animId: null
  };

  const rgba = (u32) =>
    `rgba(${(u32 >>> 24) & 255}, ${(u32 >>> 16) & 255}, ${(u32 >>> 8) & 255}, ${((u32 & 255) / 255).toFixed(2)})`;

  // ── SPACE — the whole window is the 5D starmap (crawl the sky).
  // CATALOG_16 rides the manifold camera; the deep field parallaxes under it.
  // Twinkle + two-pass dim/glow ported from v2 sky_verb.rs:168-257.
  const spaceCanvas = $('space-canvas');
  const spaceCtx = spaceCanvas.getContext('2d');
  // tx/ty/tz = the focal target: (0,0,0) is Sol, but the camera is NOT stuck
  // there — right-drag pans focus, double-click a star flies to it,
  // double-click the void returns to Sol.
  const cam5d = { distance: 220.0, pitch: 0.35, yaw: 0.0, roll: 0.0, fov_deg: 55.0, tx: 0, ty: 0, tz: 0 };
  // Free-flowing flight (Sean 2026-08-26: "free flowing as light"): drags
  // leave angular momentum, the wheel throws forward velocity along the gaze,
  // and everything decays — the camera coasts instead of snapping.
  const vel = { yaw: 0, pitch: 0, fwd: 0 };
  let flyTo = null; // eased focus flight {from:[x,y,z], to:[x,y,z], t0, dur}
  // Minimum-jerk curve — same u(t)=6t^5-15t^4+10t^3 as camera5d.rs::quintic.
  const quintic = (t) => { const u = Math.min(Math.max(t, 0), 1); return u * u * u * (u * (u * 6 - 15) + 10); };
  function integrateFlight(tMs) {
    let moved = false;
    if (flyTo) {
      const u = quintic((tMs - flyTo.t0) / flyTo.dur);
      cam5d.tx = flyTo.from[0] + (flyTo.to[0] - flyTo.from[0]) * u;
      cam5d.ty = flyTo.from[1] + (flyTo.to[1] - flyTo.from[1]) * u;
      cam5d.tz = flyTo.from[2] + (flyTo.to[2] - flyTo.from[2]) * u;
      if (u >= 1) flyTo = null;
      moved = true;
    }
    if (Math.abs(vel.yaw) + Math.abs(vel.pitch) > 1e-4 || Math.abs(vel.fwd) > 1e-3) {
      cam5d.yaw += vel.yaw;
      cam5d.pitch += vel.pitch;
      const cp = Math.cos(cam5d.pitch);
      cam5d.tx -= cp * Math.sin(cam5d.yaw) * vel.fwd;
      cam5d.ty -= Math.sin(cam5d.pitch) * vel.fwd;
      cam5d.tz -= cp * Math.cos(cam5d.yaw) * vel.fwd;
      vel.yaw *= 0.90; vel.pitch *= 0.90; vel.fwd *= 0.92;
      moved = true;
    }
    if (moved) scheduleRefreshSky();
  }
  let spaceStars = null;
  // mag 6.5 in permyriad — all naked-eye visible catalog stars
  const LORE_MAG_PMY = 65_000;
  let spaceDrawList = [];
  let skyBusy = false;
  let skyLstDeg = 0;
  let activeStarIdx = -1;
  let audioCtx = null;

  // Blinky-blinky rides the harmonics lane: per-star pitch classes octave-
  // reduced to sub-audio pulses (get_blink_score / forge-harmonics starmonics)
  // — Sirius conducts the deep field, each lore star blinks its own note.
  const blink = { beatHz: 0.9, starHz: [] };
  (async () => {
    const s = await invokeCommand('get_blink_score');
    if (s) {
      blink.beatHz = s.beat_mhz / 1000;
      blink.starHz = s.star_blink_mhz.map((m) => m / 1000);
    }
  })();
  const starBlinkHz = (idx) => blink.starHz[idx % 16] || 0.9;

  // Deterministic LCG deep field — the same sky every boot, no Math.random.
  const FIELD = (() => {
    let seed = 0x13F0;
    const next = () => ((seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
    const pts = [];
    for (let i = 0; i < 300; i++) {
      pts.push({ u: next(), v: next(), depth: 0.25 + next() * 0.75, phase: next() * 2 });
    }
    return pts;
  })();

  const triWave = (t) => { const f = t % 2; return f < 1 ? f : 2 - f; };

  // ── GL DEEP SKY — the full HYG v4.4 catalog (119,613 stars) through the
  // 5D camera, WebGL2 points + bloom + ENB godrays. Donors: byte format
  // shell/src/celestial_hyg.rs; godray march forge-ocular-v3/enb_godrays.wgsl.
  const STAR_SPHERE_R = 60.0;
  const BAYER8 = [
    0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26,
    12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22,
    3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25,
    15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21,
  ].map((v) => (v / 64).toFixed(6)).join(',');

  const STAR_VS = `#version 300 es
precision highp float;
layout(location=0) in vec3 a_pos;
layout(location=1) in vec3 a_col;
layout(location=2) in float a_bright;
layout(location=3) in float a_phase;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform vec3 u_gaze;
uniform float u_lst;
uniform float u_time;
uniform float u_beat;
uniform float u_theta_zw;
uniform float u_phi_wv;
uniform float u_beta_lorentz;
out vec3 v_col;
out float v_alpha;
out float v_bright;
out float v_flare;
out float v_seeing;
float triwave(float t){ float f = mod(t, 2.0); return f < 1.0 ? f : 2.0 - f; }
void main(){
  bool sol = a_phase < 0.0;
  float c = cos(u_lst), s = sin(u_lst);
  vec3 p0 = sol ? a_pos : vec3(a_pos.x*c + a_pos.z*s, a_pos.y, -a_pos.x*s + a_pos.z*c);

  // 1. Relativistic Lorentz Aberration Warp & Doppler Beaming
  vec3 p_rel = p0 - u_eye;
  float r_dist = length(p_rel);
  vec3 dir = r_dist > 1e-4 ? p_rel / r_dist : vec3(0.0, 0.0, 1.0);
  vec3 gz = length(u_gaze) > 1e-4 ? normalize(u_gaze) : vec3(0.0, 0.0, -1.0);
  float beta = clamp(u_beta_lorentz, 0.0, 0.95);
  float gamma = 1.0 / sqrt(max(1.0 - beta * beta, 0.01));
  float cos_a = dot(dir, gz);
  float denom = max(1.0 - beta * cos_a, 1e-4);
  float cos_a_prime = (cos_a - beta) / denom;
  float doppler = 1.0 / (gamma * denom);
  vec3 dir_perp = dir - cos_a * gz;
  vec3 warped_dir = cos_a_prime * gz + (dir_perp / (gamma * denom));
  vec3 p_warped = u_eye + normalize(warped_dir) * r_dist;

  // 2. SO(5) Givens Hyperplane Rotations (zw and wv)
  float raw_w = (1.0 - a_bright) * 12.0;
  float raw_v = a_phase;

  // R_zw(theta)
  float cz = cos(u_theta_zw), sz = sin(u_theta_zw);
  float rot_z = p_warped.z * cz - raw_w * sz;
  float rot_w = p_warped.z * sz + raw_w * cz;

  // R_wv(phi)
  float cw = cos(u_phi_wv), sw = sin(u_phi_wv);
  float rot_w2 = rot_w * cw - raw_v * sw;
  float rot_v = rot_w * sw + raw_v * cw;

  // Combined Effective Depth Projection
  float eff_z = rot_z + 0.5 * rot_w2 + 0.2 * rot_v;
  vec3 p = vec3(p_warped.x, p_warped.y, eff_z);

  gl_Position = u_vp * vec4(p, 1.0);
  float tw = sol ? 1.0 : 0.72 + 0.28*triwave(u_time*u_beat + rot_v);
  float att = clamp(240.0 / max(length(p - u_eye), 8.0), 0.7, 2.2);
  // Saliency beaming & Fredholm 1st Kind operator with Tikhonov regularized clamp
  float tikhonov_clamp = clamp(doppler, 0.2, 5.0);
  float flare = smoothstep(0.48, 0.95, a_bright * tikhonov_clamp);
  float airy_blur = 1.0 + abs(rot_w2) * 0.35;
  float sz = max(1.8, (1.0 + 13.5*pow(a_bright * tikhonov_clamp, 1.4)) * att * (1.0 + 1.4*flare) * airy_blur);
  gl_PointSize = (sol ? 36.0 : sz) * tw;

  // 3. VAST SPECTRAL CHROMATIC EXPANSION & RELATIVISTIC TRANSLATION
  // Expand raw a_col into deep, vibrant Planckian spectral bands (Crimson, Amber, Gold, Cyan, Violet, Electric White)
  vec3 base_col = a_col;
  float is_warm = max(0.0, base_col.r - base_col.b);
  float is_cool = max(0.0, base_col.b - base_col.r);
  vec3 rich_chroma = vec3(
    base_col.r * (1.0 + 0.95 * is_warm),
    base_col.g * (1.0 + 0.25 * is_warm + 0.35 * is_cool),
    base_col.b * (1.0 + 1.45 * is_cool)
  );
  // Relativistic Doppler spectral transposition & phase shift
  vec3 doppler_shift = mix(
    rich_chroma * vec3(1.5, 0.75, 0.35),  // Redshifted receding tail
    rich_chroma * vec3(0.45, 0.95, 1.75),  // Blueshifted approaching apex
    clamp((tikhonov_clamp - 0.7) * 1.2, 0.0, 1.0)
  );
  vec3 col_shift = doppler_shift * vec3(1.0 + 0.35*sin(rot_v), 1.0 + 0.18*cos(rot_v), 1.0 - 0.35*sin(rot_v));
  v_col = clamp(col_shift, 0.05, 1.8);
  v_alpha = (sol ? 1.0 : (0.32 + 0.68*a_bright) / (1.0 + abs(rot_w2)*0.1)) * tw;
  v_bright = sol ? 1.0 : a_bright * tikhonov_clamp;
  v_flare = sol ? 1.0 : flare;
  v_seeing = 0.82 + 0.18 * sin(u_time * 3.7 + rot_v * 6.2831);
}`;

  // Optics, not discs: Airy core + 4-point diffraction cross + corona.
  // Preserves full chromatic saturation across the star sprite body and halos.
  const STAR_FS = `#version 300 es
precision mediump float;
in vec3 v_col;
in float v_alpha;
in float v_bright;
in float v_flare;
in float v_seeing;
out vec4 o;
void main(){
  vec2 p = (gl_PointCoord - 0.5) * 2.0;
  float r2 = dot(p, p);
  if (r2 > 1.0) discard;
  // Deep Airy core: broad chromatic body, tight microscopic white-hot pinhole
  float core = exp(-r2 * 6.5);
  float pinhole = exp(-r2 * 28.0);
  float lit = core;
  if (v_flare > 0.002) {
    // Cross flare: a sharp lobe along each axis, damped across the other
    float sx = exp(-abs(p.x) * 20.0) * exp(-abs(p.y) * 2.5);
    float sy = exp(-abs(p.y) * 20.0) * exp(-abs(p.x) * 2.5);
    float corona = exp(-sqrt(r2) * 2.6) * 0.45;
    lit += ((sx + sy) * 0.85 * v_seeing + corona) * v_flare;
  }
  lit *= (1.0 - r2 * 0.12);
  // Vivid chromatic body: only the innermost microscopic pinhole burns white-hot for blazes
  float white_pin = smoothstep(0.70, 0.98, v_bright) * pinhole;
  vec3 col = mix(v_col, vec3(1.0), white_pin * 0.75);
  // Harmonic Superbloom with rich chromatic saturation
  float heat = 1.0 + 2.8 * smoothstep(0.68, 1.0, v_bright);
  o = vec4(col * (v_alpha * lit * heat), 1.0);
}`;

  // DS9 drift motes: a wrapping dust cloud around the EYE — the deep dome
  // never parallaxes, these are what make flight feel like moving.
  const DUST_VS = `#version 300 es
precision highp float;
layout(location=0) in vec3 a_pos;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform float u_time;
out float v_a;
void main(){
  vec3 rel = mod(a_pos - u_eye + 120.0, 240.0) - 120.0;
  float d = length(rel);
  gl_Position = u_vp * vec4(u_eye + rel, 1.0);
  gl_PointSize = clamp(150.0 / max(d, 1.0), 0.7, 5.0);
  v_a = smoothstep(120.0, 28.0, d) * (0.09 + 0.05 * sin(u_time * 0.7 + a_pos.x));
}`;

  const DUST_FS = `#version 300 es
precision mediump float;
in float v_a;
out vec4 o;
void main(){
  vec2 dd = gl_PointCoord - 0.5;
  float r2 = dot(dd, dd) * 4.0;
  o = vec4(vec3(0.55, 0.62, 0.75) * (v_a * exp(-r2*2.5) * step(r2, 1.0)), 1.0);
}`;

  const QUAD_VS = `#version 300 es
precision highp float;
out vec2 v_uv;
void main(){
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  v_uv = p;
  gl_Position = vec4(p*2.0 - 1.0, 0.0, 1.0);
}`;

  const BLUR_FS = `#version 300 es
precision mediump float;
in vec2 v_uv;
out vec4 o;
uniform sampler2D u_tex;
uniform vec2 u_dir;
uniform float u_thresh;
void main(){
  float w[5] = float[5](0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
  vec3 acc = vec3(0.0);
  for (int i = -4; i <= 4; i++) {
    vec3 c = texture(u_tex, v_uv + u_dir*float(i)).rgb;
    if (u_thresh > 0.0) c = max(c - vec3(u_thresh), vec3(0.0));
    acc += c * w[abs(i)];
  }
  o = vec4(acc, 1.0);
}`;

  // fs_main of enb_godrays.wgsl, line-for-line (32 samples, decay early-exit),
  // but marched in its OWN half-res pass: 32 dependent fetches per pixel at
  // full res was the frame's roofline (~120M fetches at 2560x1440). Quarter
  // the pixels, same march, linear upsample in the composite.
  const RAY_FS = `#version 300 es
precision highp float;
in vec2 v_uv;
out vec4 o;
uniform sampler2D u_scene;
uniform vec2 u_light;
uniform float u_density;
uniform float u_weight;
uniform float u_decay;
uniform float u_ray_on;
void main(){
  if (u_ray_on < 0.5) { o = vec4(0.0); return; }
  vec2 tc = v_uv;
  vec4 rays = vec4(0.0);
  vec2 dtc = (tc - u_light) * (1.0/32.0) * u_density;
  float dec = 1.0;
  for (int i = 0; i < 32; i++) {
    tc -= dtc;
    rays += texture(u_scene, tc) * dec * u_weight;
    dec *= u_decay;
    if (dec < 0.001) { break; }
  }
  o = rays;
}`;

  // The composite reads the marched rays instead of marching them, plus the
  // bayer8 dither mask and the old-master ground.
  const COMPOSITE_FS = `#version 300 es
precision highp float;
in vec2 v_uv;
out vec4 o;
uniform sampler2D u_scene;
uniform sampler2D u_bloom;
uniform sampler2D u_rays;
uniform vec2 u_light;
uniform float u_exposure;
uniform float u_glaze;
uniform vec4 u_haze;
uniform sampler2D u_dust;
uniform mat4 u_ivp;
uniform float u_lstr;
uniform float u_dust_on;
const float BAYER[64] = float[64](${BAYER8});
void main(){
  vec4 color = texture(u_scene, v_uv);
  vec4 fin = texture(u_rays, v_uv) * u_exposure * u_haze;
  ivec2 pp = ivec2(gl_FragCoord.xy);
  float dth = BAYER[(pp.y % 8)*8 + (pp.x % 8)];
  float lum = dot(fin.rgb, vec3(0.299, 0.587, 0.114));
  float mask = (lum * u_glaze) > (dth * 0.4) ? 1.0 : 0.0;
  // Old-master monochromatic ground (Sean: "the old masters never used
  // black"): MOLTEN soot->ash umber ramp + a soft vignette so the frame has
  // boundaries; the lights are built ON the ground, never on void.
  // Sean 2026-08-27: BLACK IS A DELINEATOR, NEVER A GROUND. Monochromatic
  // dark in the old-master sense — violet-black shoulder into green-black
  // depth, so the void has a hue to read against and stars sit ON something.
  vec3 deep   = vec3(0.035, 0.043, 0.039);
  vec3 moss   = vec3(0.051, 0.075, 0.067);
  vec3 violet = vec3(0.082, 0.059, 0.110);
  vec2 e = v_uv - 0.5;
  float vig = 1.0 - dot(e, e) * 1.35;
  vec3 ground = mix(deep, mix(moss, violet, smoothstep(0.30, 1.0, 1.0 - v_uv.y)), max(vig, 0.25));
  ground += u_haze.rgb * 0.012 / (dot(v_uv - u_light, v_uv - u_light) * 6.0 + 0.35);
  // NASA Deep Star Maps 2020 Milky Way (public domain), equirect-sampled by
  // per-pixel view ray, LST-unrotated, ENB celestial godray & bloom grading.
  if (u_dust_on > 0.5) {
    vec2 ndc = v_uv * 2.0 - 1.0;
    vec4 pf = u_ivp * vec4(ndc, 1.0, 1.0);
    vec4 pn = u_ivp * vec4(ndc, -1.0, 1.0);
    vec3 dir = normalize(pf.xyz / pf.w - pn.xyz / pn.w);
    float cl = cos(u_lstr), sl = sin(u_lstr);
    vec3 dd = vec3(dir.x*cl - dir.z*sl, dir.y, dir.x*sl + dir.z*cl);
    float ra = atan(dd.z, dd.x);
    float dec = asin(clamp(dd.y, -1.0, 1.0));
    vec2 uv_mw = vec2(ra / 6.28318530718 + 0.5, 0.5 - dec / 3.14159265359);
    vec3 dustc = texture(u_dust, uv_mw).rgb;
    
    // ENB Tone Curve & Chromatic Radiance: restore vivid galactic spine, golden core & H-alpha nebulosity
    float dlum = dot(dustc, vec3(0.299, 0.587, 0.114));
    vec3 core_tint = vec3(1.20, 0.95, 0.65);   // Golden galactic core starlight
    vec3 dust_tint = vec3(0.45, 0.60, 0.85);   // Interstellar scatter & blue reflection
    vec3 h_alpha   = vec3(1.00, 0.40, 0.65);   // Interstellar hydrogen emission
    
    float core_w = smoothstep(0.15, 0.85, dlum);
    float mid_w  = smoothstep(0.03, 0.45, dlum);
    
    vec3 enb_mw = dustc * 1.40 + 
                  core_tint * (core_w * 0.85) + 
                  dust_tint * (mid_w * 0.40) + 
                  h_alpha * (pow(dustc.r, 2.0) * 0.30);
                  
    ground += enb_mw * 0.95;
  }
  vec3 comp = ground + color.rgb + texture(u_bloom, v_uv).rgb * 1.15 + fin.rgb * (0.6 + 0.4*mask);
  o = vec4(comp, 1.0);
}`;

  const glSky = {
    ready: false, gl: null, canvas: null,
    progStars: null, progBlur: null, progComp: null,
    progDust: null, dustVao: null, dustCount: 0, dustTex: null, dustOn: 0, invVP: null,
    vao: null, starCount: 0,
    sceneFbo: null, sceneTex: null, bloomFboA: null, bloomTexA: null, bloomFboB: null, bloomTexB: null,
    w: 0, h: 0, viewProj: null, lstRad: 0, eye: [0, 0, 66],
    u: {},
  };

  // General 4x4 inverse (column-major flat), for the composite's ray cast.
  function mat4Invert(m) {
    const inv = new Float32Array(16);
    inv[0] = m[5]*m[10]*m[15] - m[5]*m[11]*m[14] - m[9]*m[6]*m[15] + m[9]*m[7]*m[14] + m[13]*m[6]*m[11] - m[13]*m[7]*m[10];
    inv[4] = -m[4]*m[10]*m[15] + m[4]*m[11]*m[14] + m[8]*m[6]*m[15] - m[8]*m[7]*m[14] - m[12]*m[6]*m[11] + m[12]*m[7]*m[10];
    inv[8] = m[4]*m[9]*m[15] - m[4]*m[11]*m[13] - m[8]*m[5]*m[15] + m[8]*m[7]*m[13] + m[12]*m[5]*m[11] - m[12]*m[7]*m[9];
    inv[12] = -m[4]*m[9]*m[14] + m[4]*m[10]*m[13] + m[8]*m[5]*m[14] - m[8]*m[6]*m[13] - m[12]*m[5]*m[10] + m[12]*m[6]*m[9];
    inv[1] = -m[1]*m[10]*m[15] + m[1]*m[11]*m[14] + m[9]*m[2]*m[15] - m[9]*m[3]*m[14] - m[13]*m[2]*m[11] + m[13]*m[3]*m[10];
    inv[5] = m[0]*m[10]*m[15] - m[0]*m[11]*m[14] - m[8]*m[2]*m[15] + m[8]*m[3]*m[14] + m[12]*m[2]*m[11] - m[12]*m[3]*m[10];
    inv[9] = -m[0]*m[9]*m[15] + m[0]*m[11]*m[13] + m[8]*m[1]*m[15] - m[8]*m[3]*m[13] - m[12]*m[1]*m[11] + m[12]*m[3]*m[9];
    inv[13] = m[0]*m[9]*m[14] - m[0]*m[10]*m[13] - m[8]*m[1]*m[14] + m[8]*m[2]*m[13] + m[12]*m[1]*m[10] - m[12]*m[2]*m[9];
    inv[2] = m[1]*m[6]*m[15] - m[1]*m[7]*m[14] - m[5]*m[2]*m[15] + m[5]*m[3]*m[14] + m[13]*m[2]*m[7] - m[13]*m[3]*m[6];
    inv[6] = -m[0]*m[6]*m[15] + m[0]*m[7]*m[14] + m[4]*m[2]*m[15] - m[4]*m[3]*m[14] - m[12]*m[2]*m[7] + m[12]*m[3]*m[6];
    inv[10] = m[0]*m[5]*m[15] - m[0]*m[7]*m[13] - m[4]*m[1]*m[15] + m[4]*m[3]*m[13] + m[12]*m[1]*m[7] - m[12]*m[3]*m[5];
    inv[14] = -m[0]*m[5]*m[14] + m[0]*m[6]*m[13] + m[4]*m[1]*m[14] - m[4]*m[2]*m[13] - m[12]*m[1]*m[6] + m[12]*m[2]*m[5];
    inv[3] = -m[1]*m[6]*m[11] + m[1]*m[7]*m[10] + m[5]*m[2]*m[11] - m[5]*m[3]*m[10] - m[9]*m[2]*m[7] + m[9]*m[3]*m[6];
    inv[7] = m[0]*m[6]*m[11] - m[0]*m[7]*m[10] - m[4]*m[2]*m[11] + m[4]*m[3]*m[10] + m[8]*m[2]*m[7] - m[8]*m[3]*m[6];
    inv[11] = -m[0]*m[5]*m[11] + m[0]*m[7]*m[9] + m[4]*m[1]*m[11] - m[4]*m[3]*m[9] - m[8]*m[1]*m[7] + m[8]*m[3]*m[5];
    inv[15] = m[0]*m[5]*m[10] - m[0]*m[6]*m[9] - m[4]*m[1]*m[10] + m[4]*m[2]*m[9] + m[8]*m[1]*m[6] - m[8]*m[2]*m[5];
    let det = m[0]*inv[0] + m[1]*inv[4] + m[2]*inv[8] + m[3]*inv[12];
    if (Math.abs(det) < 1e-12) return null;
    det = 1.0 / det;
    for (let i = 0; i < 16; i++) inv[i] *= det;
    return inv;
  }

  function mat4Perspective(fovRad, aspect, near, far) {
    const f = 1.0 / Math.tan(fovRad / 2);
    const nf = 1.0 / (near - far);
    const out = new Float32Array(16);
    out[0] = f / aspect;
    out[5] = f;
    out[10] = (far + near) * nf;
    out[11] = -1.0;
    out[14] = (2 * far * near) * nf;
    return out;
  }

  function mat4LookAtRh(eye, target, up) {
    let zx = eye[0] - target[0], zy = eye[1] - target[1], zz = eye[2] - target[2];
    let lenZ = Math.hypot(zx, zy, zz) || 1.0;
    zx /= lenZ; zy /= lenZ; zz /= lenZ;

    let xx = up[1] * zz - up[2] * zy;
    let xy = up[2] * zx - up[0] * zz;
    let xz = up[0] * zy - up[1] * zx;
    let lenX = Math.hypot(xx, xy, xz) || 1.0;
    xx /= lenX; xy /= lenX; xz /= lenX;

    let yx = zy * xz - zz * xy;
    let yy = zz * xx - zx * xz;
    let yz = zx * xy - zy * xx;

    const out = new Float32Array(16);
    out[0] = xx; out[4] = xy; out[8] = xz; out[12] = -(xx * eye[0] + xy * eye[1] + xz * eye[2]);
    out[1] = yx; out[5] = yy; out[9] = yz; out[13] = -(yx * eye[0] + yy * eye[1] + yz * eye[2]);
    out[2] = zx; out[6] = zy; out[10] = zz; out[14] = -(zx * eye[0] + zy * eye[1] + zz * eye[2]);
    out[15] = 1.0;
    return out;
  }

  function mat4Multiply(a, b) {
    const out = new Float32Array(16);
    for (let r = 0; r < 4; r++) {
      for (let c = 0; c < 4; c++) {
        out[c * 4 + r] =
          a[r] * b[c * 4] +
          a[4 + r] * b[c * 4 + 1] +
          a[8 + r] * b[c * 4 + 2] +
          a[12 + r] * b[c * 4 + 3];
      }
    }
    return out;
  }

  function transformStar5d(x, y, z, bright, phase, isSol, eye, gaze, cl, sl, thetaZw, phiWv, beta) {
    let p0x = isSol ? x : (x * cl + z * sl);
    let p0y = y;
    let p0z = isSol ? z : (-x * sl + z * cl);

    // 1. Relativistic Lorentz Aberration Warp
    const relX = p0x - eye[0], relY = p0y - eye[1], relZ = p0z - eye[2];
    const rDist = Math.hypot(relX, relY, relZ) || 1.0;
    const dir = [relX / rDist, relY / rDist, relZ / rDist];
    const gz = Math.hypot(gaze[0], gaze[1], gaze[2]) > 1e-4 ? gaze : [0, 0, -1];
    const cosA = dir[0] * gz[0] + dir[1] * gz[1] + dir[2] * gz[2];
    const betaClamp = Math.min(0.95, Math.max(0.0, beta));
    const gamma = 1.0 / Math.sqrt(Math.max(0.01, 1.0 - betaClamp * betaClamp));
    const denom = Math.max(1e-4, 1.0 - betaClamp * cosA);
    const cosAPrime = (cosA - betaClamp) / denom;
    const doppler = 1.0 / (gamma * denom);
    const perp = [dir[0] - cosA * gz[0], dir[1] - cosA * gz[1], dir[2] - cosA * gz[2]];
    const factor = 1.0 / (gamma * denom);
    const wu = [
      cosAPrime * gz[0] + perp[0] * factor,
      cosAPrime * gz[1] + perp[1] * factor,
      cosAPrime * gz[2] + perp[2] * factor
    ];
    const wuLen = Math.hypot(wu[0], wu[1], wu[2]) || 1.0;
    const pWarpX = eye[0] + (wu[0] / wuLen) * rDist;
    const pWarpY = eye[1] + (wu[1] / wuLen) * rDist;
    const pWarpZ = eye[2] + (wu[2] / wuLen) * rDist;

    // 2. SO(5) Givens Hyperplane Rotations
    const rawW = (1.0 - bright) * 12.0;
    const rawV = phase;
    const cz = Math.cos(thetaZw), sz = Math.sin(thetaZw);
    const rotZ = pWarpZ * cz - rawW * sz;
    const rotW = pWarpZ * sz + rawW * cz;
    const cw = Math.cos(phiWv), sw = Math.sin(phiWv);
    const rotW2 = rotW * cw - rawV * sw;
    const rotV = rotW * sw + rawV * cw;
    const effZ = rotZ + 0.5 * rotW2 + 0.2 * rotV;

    return {
      x: pWarpX, y: pWarpY, z: effZ,
      doppler, rotV, rotW2
    };
  }

  function updateLocalCamera() {
    const cp = Math.cos(cam5d.pitch);
    const eye = [
      cam5d.tx + cp * Math.sin(cam5d.yaw) * cam5d.distance,
      cam5d.ty + Math.sin(cam5d.pitch) * cam5d.distance,
      cam5d.tz + cp * Math.cos(cam5d.yaw) * cam5d.distance,
    ];
    const target = [cam5d.tx, cam5d.ty, cam5d.tz];
    const up = [0, 1, 0];
    const aspect = spaceCanvas.width / Math.max(1, spaceCanvas.height);
    const fovRad = (cam5d.fov_deg * Math.PI) / 180;

    const proj = mat4Perspective(fovRad, aspect, 0.1, 10000.0);
    const view = mat4LookAtRh(eye, target, up);
    glSky.viewProj = mat4Multiply(proj, view);
    glSky.invVP = null;
    glSky.eye = eye;
    glSky.lstRad = (skyLstDeg * Math.PI) / 180;

    // 2D screenspace sync on every animation frame:
    if (spaceStars) {
      const m = glSky.viewProj;
      const w = spaceCanvas.width, h = spaceCanvas.height;
      const cl = Math.cos(glSky.lstRad), sl = Math.sin(glSky.lstRad);
      const gx = cam5d.tx - eye[0], gy = cam5d.ty - eye[1], gz = cam5d.tz - eye[2];
      const glen = Math.hypot(gx, gy, gz) || 1.0;
      const gazeVec = [gx / glen, gy / glen, gz / glen];

      for (const s of spaceStars) {
        const brightNorm = Math.max(0, Math.min(1, 1.0 - (s.mag_pmy || 0) / 65000));
        const p5d = transformStar5d(
          s.wx, s.wy, s.wz, brightNorm, (s.idx % 7) * 0.9, false,
          eye, gazeVec, cl, sl, so5State.theta_zw, so5State.phi_wv, so5State.beta_lorentz
        );
        const cw = m[3] * p5d.x + m[7] * p5d.y + m[11] * p5d.z + m[15];
        if (cw <= 0.01) { s.visible = false; continue; }
        s.visible = true;
        s.sx = (m[0] * p5d.x + m[4] * p5d.y + m[8] * p5d.z + m[12]) / cw;
        s.sy = (m[1] * p5d.x + m[5] * p5d.y + m[9] * p5d.z + m[13]) / cw;
        s.depth = (m[2] * p5d.x + m[6] * p5d.y + m[10] * p5d.z + m[14]) / cw;
        s._px = (0.5 + 0.5 * s.sx) * w;
        s._py = (0.5 - 0.5 * s.sy) * h;
        s.doppler = p5d.doppler;
      }
    }
  }

  function glCompile(gl, vsSrc, fsSrc) {
    const mk = (type, src) => {
      const sh = gl.createShader(type);
      gl.shaderSource(sh, src);
      gl.compileShader(sh);
      if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(sh));
      return sh;
    };
    const p = gl.createProgram();
    gl.attachShader(p, mk(gl.VERTEX_SHADER, vsSrc));
    gl.attachShader(p, mk(gl.FRAGMENT_SHADER, fsSrc));
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
    return p;
  }

  function glTarget(gl, w, h) {
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const fbo = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    return { fbo, tex };
  }

  // The sky VBO is PREBAKED Rust-side (get_sky_vbo, main.rs bake_sky_vbo —
  // distance-true positions, saturated Teff colour, visibility, phase; the
  // float boundary lives there). This just points typed views at the wire.
  function viewSkyVbo(buf) {
    if (buf.byteLength < 4) return null;
    const n = new DataView(buf).getUint32(0, true);
    if (n === 0 || buf.byteLength < 4 + n * 44 + n * 4) return null;
    let off = 4;
    const verts = new Float32Array(buf, off, n * 8); off += n * 32;
    const ra = new Float32Array(buf, off, n); off += n * 4;
    const dec = new Float32Array(buf, off, n); off += n * 4;
    const mag = new Float32Array(buf, off, n); off += n * 4;
    const dist = new Uint16Array(buf, off, n); off += n * 2;
    const teff = new Uint8Array(buf, off, n); off += n;
    const lore = new Uint8Array(buf, off, n);
    return { verts, meta: { ra, dec, mag, dist, teff, lore } };
  }

  function glSkyResize() {
    const g = glSky;
    if (!g.gl) return;
    const w = window.innerWidth, h = window.innerHeight;
    if (w === g.w && h === g.h) return;
    g.w = w; g.h = h;
    g.canvas.width = w; g.canvas.height = h;
    const gl = g.gl;
    for (const t of [g.sceneFbo, g.bloomFboA, g.bloomFboB, g.rayFbo]) {
      if (t) { gl.deleteFramebuffer(t.fbo); gl.deleteTexture(t.tex); }
    }
    g.sceneFbo = glTarget(gl, w, h);
    const hw = Math.max(1, w >> 1), hh = Math.max(1, h >> 1);
    g.bloomFboA = glTarget(gl, hw, hh);
    g.bloomFboB = glTarget(gl, hw, hh);
    g.rayFbo = glTarget(gl, hw, hh);
  }

  async function glSkyBoot() {
    try {
      const bytes = await invokeCommand('get_sky_vbo');
      if (!bytes) return;
      const buf = bytes instanceof ArrayBuffer ? bytes : new Uint8Array(bytes).buffer;
      const decoded = viewSkyVbo(buf);
      if (!decoded) { console.warn('[shell] sky vbo refused: bad bytes'); return; }
      const verts = decoded.verts;
      glSky.pick = verts;
      glSky.meta = decoded.meta;
      const canvas = $('gl-sky');
      const gl = canvas.getContext('webgl2', { alpha: false, antialias: false, depth: false });
      if (!gl) { glSky.failed = true; console.warn('[shell] WebGL2 unavailable — LCG field stays'); return; }
      glSky.canvas = canvas;
      glSky.gl = gl;
      glSky.progStars = glCompile(gl, STAR_VS, STAR_FS);
      glSky.progBlur = glCompile(gl, QUAD_VS, BLUR_FS);
      glSky.progComp = glCompile(gl, QUAD_VS, COMPOSITE_FS);
      glSky.progRay = glCompile(gl, QUAD_VS, RAY_FS);
      glSky.vao = gl.createVertexArray();
      gl.bindVertexArray(glSky.vao);
      const vbo = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
      gl.bufferData(gl.ARRAY_BUFFER, verts, gl.STATIC_DRAW);
      const stride = 8 * 4;
      gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 3, gl.FLOAT, false, stride, 0);
      gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 3, gl.FLOAT, false, stride, 12);
      gl.enableVertexAttribArray(2); gl.vertexAttribPointer(2, 1, gl.FLOAT, false, stride, 24);
      gl.enableVertexAttribArray(3); gl.vertexAttribPointer(3, 1, gl.FLOAT, false, stride, 28);
      gl.bindVertexArray(null);
      glSky.starCount = verts.length / 8;
      // DS9 drift motes: deterministic LCG cloud, wraps around the eye.
      glSky.progDust = glCompile(gl, DUST_VS, DUST_FS);
      let mseed = 0xD59;
      const mnext = () => ((mseed = (mseed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
      const motes = new Float32Array(6000 * 3);
      for (let i = 0; i < motes.length; i++) motes[i] = mnext() * 240 - 120;
      glSky.dustVao = gl.createVertexArray();
      gl.bindVertexArray(glSky.dustVao);
      const dvbo = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, dvbo);
      gl.bufferData(gl.ARRAY_BUFFER, motes, gl.STATIC_DRAW);
      gl.enableVertexAttribArray(0);
      gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 12, 0);
      gl.bindVertexArray(null);
      glSky.dustCount = 6000;
      // NASA Milky Way equirect (ui/milkyway.jpg, public domain).
      const dimg = new Image();
      dimg.crossOrigin = 'anonymous';
      dimg.onload = () => {
        try {
          const tx = gl.createTexture();
          gl.bindTexture(gl.TEXTURE_2D, tx);
          gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
          gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, dimg);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
          glSky.dustTex = tx;
          glSky.dustOn = 1;
          console.log('[shell] NASA Milky Way ENB panorama loaded successfully');
        } catch (texErr) {
          console.error('[shell] Error uploading milkyway texture to WebGL:', texErr);
        }
      };
      dimg.onerror = (err) => {
        console.warn('[shell] Failed loading milkyway.jpg directly, trying ./milkyway.jpg:', err);
        if (!dimg.src.endsWith('./milkyway.jpg')) {
          dimg.src = './milkyway.jpg';
        }
      };
      dimg.src = 'milkyway.jpg';
      const u = (p, n) => gl.getUniformLocation(p, n);
      glSky.u = {
        vp: u(glSky.progStars, 'u_vp'), eye: u(glSky.progStars, 'u_eye'), gaze: u(glSky.progStars, 'u_gaze'), lst: u(glSky.progStars, 'u_lst'), time: u(glSky.progStars, 'u_time'), beat: u(glSky.progStars, 'u_beat'),
        theta_zw: u(glSky.progStars, 'u_theta_zw'), phi_wv: u(glSky.progStars, 'u_phi_wv'), beta_lorentz: u(glSky.progStars, 'u_beta_lorentz'),
        blurTex: u(glSky.progBlur, 'u_tex'), blurDir: u(glSky.progBlur, 'u_dir'), blurThresh: u(glSky.progBlur, 'u_thresh'),
        dvp: u(glSky.progDust, 'u_vp'), deye: u(glSky.progDust, 'u_eye'), dtime: u(glSky.progDust, 'u_time'),
        dust: u(glSky.progComp, 'u_dust'), ivp: u(glSky.progComp, 'u_ivp'), lstr: u(glSky.progComp, 'u_lstr'), dustOn: u(glSky.progComp, 'u_dust_on'),
        scene: u(glSky.progComp, 'u_scene'), bloom: u(glSky.progComp, 'u_bloom'), light: u(glSky.progComp, 'u_light'),
        rays: u(glSky.progComp, 'u_rays'),
        exposure: u(glSky.progComp, 'u_exposure'), glaze: u(glSky.progComp, 'u_glaze'),
        haze: u(glSky.progComp, 'u_haze'),
        rScene: u(glSky.progRay, 'u_scene'), rLight: u(glSky.progRay, 'u_light'),
        rDensity: u(glSky.progRay, 'u_density'), rWeight: u(glSky.progRay, 'u_weight'),
        rDecay: u(glSky.progRay, 'u_decay'), rOn: u(glSky.progRay, 'u_ray_on'),
      };
      glSkyResize();
      glSky.ready = true;
      document.title = `Forge V3 Demo Shell | GL OK ${glSky.starCount}`;
      console.log(`[shell] HYG deep sky online: ${glSky.starCount} stars (Sol restored at origin)`);
    } catch (e) {
      // The window title is the only channel out of this webview that a
      // capture tool can read without devtools — the GL refusal has been
      // invisible, and invisible is why the sky has been dead.
      document.title = 'GLFAIL: ' + String(e && e.message ? e.message : e).replace(/\s+/g, ' ').slice(0, 170);
      console.error('[shell] gl-sky refused:', e);
    }
  }

  // Shaft origin: Sol, the orbit target — its projection rides the vp's
  // 4th column.
  function glLightUv(tMs) {
    const m = glSky.viewProj;
    if (m && m[15] > 1e-6) return [0.5 + 0.5 * (m[12] / m[15]), 0.5 + 0.5 * (m[13] / m[15])];
    return null;
  }

  function glSkyDraw(tMs) {
    const g = glSky;
    if (!g.ready || !g.viewProj) return;
    glSkyResize();
    const gl = g.gl;
    const t = tMs / 1000;

    const gx = cam5d.tx - g.eye[0], gy = cam5d.ty - g.eye[1], gz = cam5d.tz - g.eye[2];
    const glen = Math.hypot(gx, gy, gz) || 1.0;
    const gazeVec = [gx / glen, gy / glen, gz / glen];

    gl.bindFramebuffer(gl.FRAMEBUFFER, g.sceneFbo.fbo);
    gl.viewport(0, 0, g.w, g.h);
    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE);
    gl.useProgram(g.progStars);
    gl.uniformMatrix4fv(g.u.vp, false, g.viewProj);
    gl.uniform3f(g.u.eye, g.eye[0], g.eye[1], g.eye[2]);
    gl.uniform3f(g.u.gaze, gazeVec[0], gazeVec[1], gazeVec[2]);
    gl.uniform1f(g.u.lst, g.lstRad);
    gl.uniform1f(g.u.time, t);
    gl.uniform1f(g.u.beat, blink.beatHz);
    gl.uniform1f(g.u.theta_zw, so5State.theta_zw);
    gl.uniform1f(g.u.phi_wv, so5State.phi_wv);
    gl.uniform1f(g.u.beta_lorentz, so5State.beta_lorentz);
    gl.bindVertexArray(g.vao);
    gl.drawArrays(gl.POINTS, 0, g.starCount);
    gl.bindVertexArray(null);
    // DS9 drift motes ride the same scene FBO, so bloom + godrays light them.
    gl.useProgram(g.progDust);
    gl.uniformMatrix4fv(g.u.dvp, false, g.viewProj);
    gl.uniform3f(g.u.deye, g.eye[0], g.eye[1], g.eye[2]);
    gl.uniform1f(g.u.dtime, t);
    gl.bindVertexArray(g.dustVao);
    gl.drawArrays(gl.POINTS, 0, g.dustCount);
    gl.bindVertexArray(null);
    gl.disable(gl.BLEND);

    const hw = Math.max(1, g.w >> 1), hh = Math.max(1, g.h >> 1);
    gl.useProgram(g.progBlur);
    gl.activeTexture(gl.TEXTURE0);
    gl.uniform1i(g.u.blurTex, 0);
    gl.bindFramebuffer(gl.FRAMEBUFFER, g.bloomFboA.fbo);
    gl.viewport(0, 0, hw, hh);
    gl.bindTexture(gl.TEXTURE_2D, g.sceneFbo.tex);
    gl.uniform2f(g.u.blurDir, 1.6 / hw, 0);
    gl.uniform1f(g.u.blurThresh, 0.55);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.bindFramebuffer(gl.FRAMEBUFFER, g.bloomFboB.fbo);
    gl.bindTexture(gl.TEXTURE_2D, g.bloomFboA.tex);
    gl.uniform2f(g.u.blurDir, 0, 1.6 / hh);
    gl.uniform1f(g.u.blurThresh, 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    // GodRayUniforms::default() (forge-ocular-v3/src/godrays.rs:38-51), marched
    // at half res into its own target — the composite only samples it.
    const light = glLightUv(tMs);
    gl.bindFramebuffer(gl.FRAMEBUFFER, g.rayFbo.fbo);
    gl.viewport(0, 0, hw, hh);
    gl.useProgram(g.progRay);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, g.sceneFbo.tex);
    gl.uniform1i(g.u.rScene, 0);
    gl.uniform2f(g.u.rLight, light ? light[0] : 0.5, light ? light[1] : 0.5);
    gl.uniform1f(g.u.rDensity, 1.0);
    gl.uniform1f(g.u.rWeight, 0.05);
    gl.uniform1f(g.u.rDecay, 0.96);
    gl.uniform1f(g.u.rOn, light ? 1.0 : 0.0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, g.w, g.h);
    gl.useProgram(g.progComp);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, g.sceneFbo.tex);
    gl.uniform1i(g.u.scene, 0);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, g.bloomFboB.tex);
    gl.uniform1i(g.u.bloom, 1);
    gl.activeTexture(gl.TEXTURE3);
    gl.bindTexture(gl.TEXTURE_2D, g.rayFbo.tex);
    gl.uniform1i(g.u.rays, 3);
    gl.uniform2f(g.u.light, light ? light[0] : 0.5, light ? light[1] : 0.5);
    gl.uniform1f(g.u.exposure, 1.2);
    gl.uniform1f(g.u.glaze, 1.0);
    gl.uniform4f(g.u.haze, 1.0, 0.95, 0.85, 1.0);
    const ivp = g.invVP || (g.invVP = mat4Invert(g.viewProj));
    if (ivp && g.dustTex) {
      gl.activeTexture(gl.TEXTURE2);
      gl.bindTexture(gl.TEXTURE_2D, g.dustTex);
      gl.uniform1i(g.u.dust, 2);
      gl.uniformMatrix4fv(g.u.ivp, false, ivp);
      gl.uniform1f(g.u.lstr, g.lstRad);
      gl.uniform1f(g.u.dustOn, g.dustOn);
    } else {
      gl.uniform1f(g.u.dustOn, 0);
    }
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  function sizeSpace() {
    spaceCanvas.width = window.innerWidth;
    spaceCanvas.height = window.innerHeight;
  }
  window.addEventListener('resize', sizeSpace);

  // ── VAST SPECTRAL HARMONIC ENGINE & CONSTELLATION ARPEGGIATOR ──
  let chainModeActive = false;
  let constellationChain = []; // [{ idx, name, milli_hz, hz, mag_pmy, color_rgba, wx, wy, wz, _px, _py, flareT }]
  let chainIsLoop = false;
  let chainPulseProgress = 0.0;
  let chainLastStepIdx = -1;
  let chainDroneOscs = [];
  let chainDroneGain = null;
  let chainBPM = 120;
  let lastChainFrameT = 0;

  function harmonicColorOfHz(hz, defaultRgba) {
    if (hz > 0) {
      // Frequency-Driven Palette:
      // Low Frequencies (100 - 400 Hz): Deep Crimson / Molten Amber / Rich Topaz
      // Mid Frequencies (400 - 1600 Hz): Solar Gold / Solar Yellow / Aquamarine
      // High Frequencies (1600 - 6400+ Hz): Electric Cyan / Royal Azure / Amethyst Violet / Starlight White
      if (hz < 220) return [245, 42, 65];        // Deep Crimson (#F52A41)
      if (hz < 330) return [255, 106, 26];       // Molten Amber (#FF6A1A)
      if (hz < 440) return [255, 175, 10];       // Rich Topaz (#FFAF0A)
      if (hz < 660) return [255, 215, 0];        // Solar Gold (#FFD700)
      if (hz < 880) return [255, 238, 88];       // Solar Yellow (#FFEE58)
      if (hz < 1320) return [72, 230, 255];      // Aquamarine (#48E6FF)
      if (hz < 1760) return [0, 229, 255];       // Electric Cyan (#00E5FF)
      if (hz < 2640) return [66, 153, 255];      // Royal Azure (#4299FF)
      if (hz < 3520) return [168, 85, 247];      // Amethyst Violet (#A855F7)
      return [240, 248, 255];                    // Electric Starlight White (#F0F8FF)
    }
    const cr = (defaultRgba >>> 24) & 255, cg = (defaultRgba >>> 16) & 255, cb = (defaultRgba >>> 8) & 255;
    return [cr, cg, cb];
  }

  function noteNameOfHz(hz) {
    if (!hz || hz <= 0) return '—';
    const noteNames = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
    const midi = Math.round(69 + 12 * Math.log2(hz / 440));
    const octave = Math.floor(midi / 12) - 1;
    const name = noteNames[(midi % 12 + 12) % 12];
    return `${name}${octave}`;
  }

  function playConstellationPulseNote(hz, dopplerFactor) {
    if (muted || !hz || hz <= 10) return;
    audioBus();
    if (audioCtx.state === 'suspended') audioCtx.resume();
    const t0 = audioCtx.currentTime;
    const dur = 0.42;

    const osc1 = audioCtx.createOscillator();
    const osc2 = audioCtx.createOscillator();
    const gainNode = audioCtx.createGain();

    osc1.type = 'sine';
    osc1.frequency.setValueAtTime(hz, t0);

    osc2.type = 'triangle';
    osc2.frequency.setValueAtTime(hz * 2 * (dopplerFactor || 1.0), t0);

    const peak = 0.12;
    gainNode.gain.setValueAtTime(0.0001, t0);
    gainNode.gain.exponentialRampToValueAtTime(peak, t0 + 0.03);
    gainNode.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);

    osc1.connect(gainNode);
    osc2.connect(gainNode);
    gainNode.connect(analyser);

    osc1.start(t0);
    osc2.start(t0);
    osc1.stop(t0 + dur + 0.05);
    osc2.stop(t0 + dur + 0.05);
  }

  function updateChainDrone() {
    if (!audioCtx) return;
    if (!chainIsLoop || constellationChain.length < 3 || muted) {
      if (chainDroneGain) {
        const t = audioCtx.currentTime;
        chainDroneGain.gain.linearRampToValueAtTime(0.0001, t + 0.3);
        setTimeout(() => {
          for (const o of chainDroneOscs) { try { o.stop(); o.disconnect(); } catch(_) {} }
          chainDroneOscs = [];
        }, 350);
      }
      return;
    }
    audioBus();
    for (const o of chainDroneOscs) { try { o.stop(); o.disconnect(); } catch(_) {} }
    chainDroneOscs = [];
    if (!chainDroneGain) {
      chainDroneGain = audioCtx.createGain();
      chainDroneGain.connect(analyser);
    }
    const t0 = audioCtx.currentTime;
    chainDroneGain.gain.setValueAtTime(0.0001, t0);
    chainDroneGain.gain.linearRampToValueAtTime(0.035, t0 + 0.5);
    for (const node of constellationChain) {
      const osc = audioCtx.createOscillator();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(node.hz, t0);
      osc.connect(chainDroneGain);
      osc.start(t0);
      chainDroneOscs.push(osc);
    }
  }

  function toggleConstellationNode(s) {
    if (!s) return;
    const existingIdx = constellationChain.findIndex((n) => n.idx === s.idx);
    if (existingIdx === 0 && constellationChain.length >= 3 && !chainIsLoop) {
      chainIsLoop = true;
      updateChainDrone();
      updateChainHud();
      return;
    }
    if (existingIdx >= 0) {
      constellationChain.splice(existingIdx, 1);
      chainIsLoop = false;
      updateChainDrone();
      updateChainHud();
      return;
    }
    const hz = s.milli_hz ? s.milli_hz / 1000 : 440;
    constellationChain.push({
      idx: s.idx,
      name: s.name,
      milli_hz: s.milli_hz,
      hz,
      mag_pmy: s.mag_pmy,
      color_rgba: s.color_rgba,
      wx: s.wx || 0,
      wy: s.wy || 0,
      wz: s.wz || 0,
      _px: s._px,
      _py: s._py,
      flareT: performance.now(),
    });
    if (constellationChain.length >= 3 && chainIsLoop) {
      updateChainDrone();
    }
    updateChainHud();
  }

  function clearConstellationChain() {
    constellationChain = [];
    chainIsLoop = false;
    chainPulseProgress = 0;
    chainLastStepIdx = -1;
    updateChainDrone();
    updateChainHud();
  }

  function updateChainHud() {
    const hud = $('chain-hud');
    if (hud) {
      if (constellationChain.length > 0) {
        hud.classList.remove('hidden');
      } else {
        hud.classList.add('hidden');
      }
    }
    const cCount = $('chain-node-count');
    if (cCount) cCount.textContent = `${constellationChain.length} nodes`;
    const cMode = $('chain-mode');
    if (cMode) {
      cMode.textContent = chainIsLoop ? 'STANDING WAVE RESONANCE' : (constellationChain.length >= 2 ? 'ARPEGGIATOR CHAIN' : 'SOLO NODE');
      cMode.style.color = chainIsLoop ? '#FFD54A' : '#7FB86A';
    }

    // Update Floating Chaining Pill
    const hintText = $('chain-hint-text');
    const btnClear = $('btn-chain-clear');
    if (constellationChain.length === 0) {
      if (hintText) hintText.textContent = '✦ Click stars to link harmonic chord';
      if (btnClear) btnClear.classList.add('hidden');
    } else if (constellationChain.length === 1) {
      const s = constellationChain[0];
      if (hintText) hintText.textContent = `✦ Linked: ${s.name} · Click next star to weave chord`;
      if (btnClear) btnClear.classList.remove('hidden');
    } else {
      const names = constellationChain.map((s) => s.name).join(' → ');
      if (hintText) hintText.textContent = `✦ ${constellationChain.length}-Star Chain: ${names}`;
      if (btnClear) btnClear.classList.remove('hidden');
    }
  }

  function drawSpace(tMs) {
    const w = spaceCanvas.width, h = spaceCanvas.height;
    const t = tMs / 1000;
    const now = performance.now();
    spaceCtx.clearRect(0, 0, w, h);
    // Celestial field — always active so background never drops to pitch black:
    for (const p of FIELD) {
      const px = ((((p.u + (cam5d.yaw * 0.05 + skyLstDeg / 360) * p.depth) % 1) + 1) % 1) * w;
      const py = ((((p.v + cam5d.pitch * 0.05 * p.depth) % 1) + 1) % 1) * h;
      const tw = 0.35 + 0.65 * triWave(t * (0.3 + p.depth) + p.phase);
      const r = 0.8 + p.depth * 1.6;
      spaceCtx.fillStyle = `rgba(247, 233, 210, ${((0.25 + 0.55 * p.depth) * tw).toFixed(3)})`;
      spaceCtx.beginPath(); spaceCtx.arc(px, py, r, 0, Math.PI * 2); spaceCtx.fill();
      if (p.depth > 0.75) {
        spaceCtx.fillStyle = `rgba(158, 200, 225, ${(0.18 * tw).toFixed(3)})`;
        spaceCtx.beginPath(); spaceCtx.arc(px, py, r * 2.4, 0, Math.PI * 2); spaceCtx.fill();
      }
    }
    if (spaceStars) {
      const cx = w / 2, cy = h / 2;
      for (const s of spaceStars) { // picking anchors for every catalog star
        if (!s.visible) { s._px = -9999; s._py = -9999; continue; }
        s._px = cx + s.sx * cx;
        s._py = cy - s.sy * cy;
        // Keep synced in constellationChain
        const cNode = constellationChain.find((n) => n.idx === s.idx);
        if (cNode) { cNode._px = s._px; cNode._py = s._py; }
      }
      for (const s of spaceDrawList) {
        if (!s.visible) continue;
        const px = s._px, py = s._py;
        if (px < -10 || px > w + 10 || py < -10 || py > h + 10) continue;
        const near = 1.0 - Math.min(1, Math.max(0, s.depth));
        const bright = Math.max(1.2, (6.5 - s.mag_pmy / 10000) * (0.8 + near * 1.5));
        const tw = 0.75 + 0.25 * triWave(t * starBlinkHz(s.idx) + s.idx * 0.37);
        const hz = s.milli_hz ? s.milli_hz / 1000 : 440;
        const [cr, cg, cb] = harmonicColorOfHz(hz, s.color_rgba);

        // Check if star node is in active excitation flare from pulse trigger
        const cNode = constellationChain.find((n) => n.idx === s.idx);
        let flareMult = 1.0;
        if (cNode && cNode.flareT) {
          const ageS = (now - cNode.flareT) / 1000;
          if (ageS < 0.6) {
            flareMult += Math.exp(-ageS * 6.0) * 2.2;
          }
        }

        const rad = Math.max(2.0, bright * 2.8 * flareMult);
        const grad = spaceCtx.createRadialGradient(px, py, 0, px, py, rad);
        grad.addColorStop(0, `rgba(255, 255, 255, ${(Math.min(1.0, tw * flareMult)).toFixed(3)})`);
        grad.addColorStop(0.25, `rgba(${cr}, ${cg}, ${cb}, ${(0.92 * tw).toFixed(3)})`);
        grad.addColorStop(0.65, `rgba(${cr}, ${cg}, ${cb}, ${(0.45 * tw).toFixed(3)})`);
        grad.addColorStop(1, `rgba(${cr}, ${cg}, ${cb}, 0)`);
        spaceCtx.globalCompositeOperation = 'lighter';
        spaceCtx.fillStyle = grad;
        spaceCtx.beginPath(); spaceCtx.arc(px, py, rad, 0, Math.PI * 2); spaceCtx.fill();
        spaceCtx.globalCompositeOperation = 'source-over';

        if (s.idx === activeStarIdx) {
          spaceCtx.strokeStyle = 'rgba(255, 106, 26, 0.9)';
          spaceCtx.lineWidth = 1.5;
          spaceCtx.beginPath(); spaceCtx.arc(px, py, rad + 6, 0, Math.PI * 2); spaceCtx.stroke();
          spaceCtx.fillStyle = '#FFD54A';
          spaceCtx.font = '11px monospace';
          spaceCtx.fillText(s.name, px + rad + 8, py + 4);
        }
      }
    }

    // ── CONSTELLATION ARPEGGIATOR CHORDS & TRAVERSING PULSES ──
    const numNodes = constellationChain.length;
    if (numNodes >= 1) {
      const numSegs = chainIsLoop ? numNodes : (numNodes - 1);

      // Draw constellation cords & standing waves
      if (numSegs > 0) {
        for (let i = 0; i < numSegs; i++) {
          const na = constellationChain[i];
          const nb = constellationChain[(i + 1) % numNodes];
          if (na._px < -100 || nb._px < -100) continue;

          const colA = harmonicColorOfHz(na.hz, na.color_rgba);
          const colB = harmonicColorOfHz(nb.hz, nb.color_rgba);

          // Vector line gradient
          const lineGrad = spaceCtx.createLinearGradient(na._px, na._py, nb._px, nb._py);
          lineGrad.addColorStop(0, `rgba(${colA[0]}, ${colA[1]}, ${colA[2]}, 0.85)`);
          lineGrad.addColorStop(1, `rgba(${colB[0]}, ${colB[1]}, ${colB[2]}, 0.85)`);

          spaceCtx.strokeStyle = lineGrad;
          spaceCtx.lineWidth = chainIsLoop ? 2.5 : 1.8;
          spaceCtx.shadowColor = `rgb(${colA[0]}, ${colA[1]}, ${colA[2]})`;
          spaceCtx.shadowBlur = chainIsLoop ? 8 : 4;

          // Standing wave oscillation along cord
          const dx = nb._px - na._px, dy = nb._py - na._py;
          const dist = Math.hypot(dx, dy);
          const steps = Math.max(4, Math.floor(dist / 8));
          const perpX = -dy / (dist || 1), perpY = dx / (dist || 1);

          spaceCtx.beginPath();
          spaceCtx.moveTo(na._px, na._py);
          for (let s = 1; s < steps; s++) {
            const frac = s / steps;
            const waveAmp = (chainIsLoop ? 4.5 : 2.0) * Math.sin(frac * Math.PI) * Math.sin(frac * 12.0 - t * 8.0);
            const wx = na._px + dx * frac + perpX * waveAmp;
            const wy = na._py + dy * frac + perpY * waveAmp;
            spaceCtx.lineTo(wx, wy);
          }
          spaceCtx.lineTo(nb._px, nb._py);
          spaceCtx.stroke();
          spaceCtx.shadowBlur = 0;
        }

        // Advance sequential light pulse
        const dt = lastChainFrameT ? Math.min(0.1, (tMs - lastChainFrameT) / 1000) : 0.016;
        lastChainFrameT = tMs;
        chainPulseProgress += dt * (chainBPM / 60) * 1.5;
        if (chainPulseProgress >= numSegs) {
          chainPulseProgress = chainIsLoop ? (chainPulseProgress % numSegs) : 0;
        }

        const curSeg = Math.floor(chainPulseProgress);
        const segFrac = chainPulseProgress - curSeg;
        const curA = constellationChain[curSeg];
        const curB = constellationChain[(curSeg + 1) % numNodes];

        // Harmonic Trigger on step crossing
        if (curSeg !== chainLastStepIdx) {
          chainLastStepIdx = curSeg;
          const dz = (curB.wz - curA.wz) || 0;
          const dx = (curB.wx - curA.wx) || 0;
          const dy = (curB.wy - curA.wy) || 0;
          const segLen = Math.hypot(dx, dy, dz) || 1;
          const cosTheta = -dz / segLen;
          const beta = Math.min(0.95, Math.max(0.0, so5State.beta_lorentz || 0.2));
          const doppler = Math.sqrt(Math.max(0.01, (1 + beta * cosTheta) / (1 - beta * cosTheta + 1e-4)));
          const shiftedHz = curA.hz * doppler;

          playConstellationPulseNote(shiftedHz, doppler);
          curA.flareT = now;

          const dopEl = $('chain-doppler');
          if (dopEl) {
            dopEl.textContent = `${doppler.toFixed(2)}x (${doppler > 1.05 ? 'BLUE-SHIFT' : (doppler < 0.95 ? 'RED-SHIFT' : '1.00x')})`;
            dopEl.style.color = doppler > 1.05 ? '#00e5ff' : (doppler < 0.95 ? '#ff6a1a' : '#ffd54a');
          }
        }

        // Draw traveling pulse bead
        if (curA._px > -100 && curB._px > -100) {
          const ppx = curA._px + (curB._px - curA._px) * segFrac;
          const ppy = curA._py + (curB._py - curA._py) * segFrac;

          const dz = (curB.wz - curA.wz) || 0;
          const isBlue = dz < 0; // Approaching
          const pCol = isBlue ? [0, 229, 255] : [255, 106, 26];

          spaceCtx.fillStyle = '#FFFFFF';
          spaceCtx.shadowColor = `rgb(${pCol[0]}, ${pCol[1]}, ${pCol[2]})`;
          spaceCtx.shadowBlur = 14;
          spaceCtx.beginPath();
          spaceCtx.arc(ppx, ppy, 4.5, 0, Math.PI * 2);
          spaceCtx.fill();
          spaceCtx.shadowBlur = 0;
        }
      }

      // Draw node rings and note badges
      for (let i = 0; i < numNodes; i++) {
        const node = constellationChain[i];
        if (node._px < -100) continue;
        const col = harmonicColorOfHz(node.hz, node.color_rgba);

        // Outer ring
        spaceCtx.strokeStyle = `rgb(${col[0]}, ${col[1]}, ${col[2]})`;
        spaceCtx.lineWidth = 1.8;
        spaceCtx.beginPath();
        spaceCtx.arc(node._px, node._py, 8.0, 0, Math.PI * 2);
        spaceCtx.stroke();

        // Node badge (Note Name + Harmonic Hz)
        spaceCtx.fillStyle = '#FFD54A';
        spaceCtx.font = 'bold 10px monospace';
        const noteTag = `${noteNameOfHz(node.hz)} (${node.hz.toFixed(0)}Hz)`;
        spaceCtx.fillText(noteTag, node._px + 12, node._py - 6);
      }
    }

    // ── EARTH (TERRA / HOME WORLD) CELESTIAL NODE ──
    const em = glSky.viewProj;
    if (em) {
      const earthAngle = t * 0.12;
      const earthDist = 18.0; // AU-scaled visual orbit around Sol
      const ex = Math.cos(earthAngle) * earthDist;
      const ey = Math.sin(earthAngle * 0.3) * 3.0;
      const ez = Math.sin(earthAngle) * earthDist;
      const cl = Math.cos(glSky.lstRad), sl = Math.sin(glSky.lstRad);
      const rx = ex * cl + ez * sl, rz = -ex * sl + ez * cl;
      const ecw = em[3] * rx + em[7] * ey + em[11] * rz + em[15];
      if (ecw > 0.01) {
        const esx = (em[0] * rx + em[4] * ey + em[8] * rz + em[12]) / ecw;
        const esy = (em[1] * rx + em[5] * ey + em[9] * rz + em[13]) / ecw;
        const epx = w / 2 + esx * (w / 2);
        const epy = h / 2 - esy * (h / 2);

        if (epx >= -30 && epx <= w + 30 && epy >= -30 && epy <= h + 30) {
          // Dynamic atmosphere glow halo
          spaceCtx.beginPath();
          spaceCtx.arc(epx, epy, 15, 0, Math.PI * 2);
          spaceCtx.strokeStyle = 'rgba(70, 190, 255, 0.75)';
          spaceCtx.lineWidth = 1.5;
          spaceCtx.stroke();

          // Reticle crosshairs
          spaceCtx.strokeStyle = 'rgba(70, 190, 255, 0.4)';
          spaceCtx.lineWidth = 1;
          spaceCtx.beginPath();
          spaceCtx.moveTo(epx - 20, epy); spaceCtx.lineTo(epx + 20, epy);
          spaceCtx.moveTo(epx, epy - 20); spaceCtx.lineTo(epx, epy + 20);
          spaceCtx.stroke();

          // Azure marble core (Ocean blue, tropical cyan, terrestrial emerald)
          const eGrad = spaceCtx.createRadialGradient(epx - 2, epy - 2, 1, epx, epy, 8);
          eGrad.addColorStop(0, '#80d8ff');
          eGrad.addColorStop(0.35, '#0288d1');
          eGrad.addColorStop(0.7, '#2e7d32');
          eGrad.addColorStop(1, '#0d47a1');

          spaceCtx.beginPath();
          spaceCtx.arc(epx, epy, 8, 0, Math.PI * 2);
          spaceCtx.fillStyle = eGrad;
          spaceCtx.shadowColor = '#00e5ff';
          spaceCtx.shadowBlur = 10;
          spaceCtx.fill();
          spaceCtx.shadowBlur = 0;

          // Label
          spaceCtx.fillStyle = '#00e5ff';
          spaceCtx.font = 'bold 11px monospace';
          spaceCtx.fillText('🌍 Terra (Earth)', epx + 16, epy + 4);
        }
      }
    }

    drawYod();
    const astroA = astroImmersion();
    if (astroA > 0.01) drawAstrolabe(astroA);
  }

  let refreshTimer = 0;
  function scheduleRefreshSky() {
    if (refreshTimer) clearTimeout(refreshTimer);
    refreshTimer = setTimeout(refreshSky, 80);
  }

  function skyFrame(tMs) {
    integrateFlight(tMs);
    updateLocalCamera();
    glSkyDraw(tMs);
    drawSpace(tMs);
    requestAnimationFrame(skyFrame);
  }

  async function refreshSky() {
    if (skyBusy) return;
    skyBusy = true;
    const payload = await invokeCommand('get_starmap_5d', {
      distance: cam5d.distance, pitch: cam5d.pitch, yaw: cam5d.yaw,
      roll: cam5d.roll, fovDeg: cam5d.fov_deg,
      aspect: spaceCanvas.width / Math.max(1, spaceCanvas.height),
      tx: cam5d.tx, ty: cam5d.ty, tz: cam5d.tz,
    });
    skyBusy = false;
    if (!payload) return;
    spaceStars = payload.stars;
    spaceDrawList = spaceStars
      .filter((s) => s.mag_pmy <= LORE_MAG_PMY)
      .sort((a, b) => b.depth - a.depth);
    skyLstDeg = payload.lst_deg || 0;
    const offSol = Math.abs(cam5d.tx) + Math.abs(cam5d.ty) + Math.abs(cam5d.tz) > 0.5;
    const focus = offSol ? `(${cam5d.tx.toFixed(0)}, ${cam5d.ty.toFixed(0)}, ${cam5d.tz.toFixed(0)})` : 'Sol';
    $('sky-title').textContent =
      `5D SKY — r=${cam5d.distance.toFixed(0)} pitch=${cam5d.pitch.toFixed(2)} yaw=${cam5d.yaw.toFixed(2)} fov=${cam5d.fov_deg.toFixed(0)}° · focus ${focus}`;
  }

  function ringStar() { ringStarIdx(activeStarIdx); }

  let analyser = null; // one tap for every ring — the dossier scope reads it
  let masterGain = null; // the ONE fader every voice passes through
  let muted = localStorage.getItem('forge.muted') === '1';
  let currentVolume = parseFloat(localStorage.getItem('forge.volume') || '0.45');

  // Every voice in this shell — star rings, the terminal's ear, a played
  // score — connects to `analyser`, so the fader goes AFTER it: the scope
  // keeps drawing what would be sounding even while muted.
  function audioBus() {
    audioCtx = audioCtx || new (window.AudioContext || window.webkitAudioContext)();
    if (audioCtx.state === 'suspended') audioCtx.resume();
    if (!analyser) {
      analyser = audioCtx.createAnalyser();
      analyser.fftSize = 512;
    }
    if (!masterGain) {
      masterGain = audioCtx.createGain();
      masterGain.gain.value = muted ? 0 : currentVolume;
      analyser.connect(masterGain);
      masterGain.connect(audioCtx.destination);
    }
    return analyser;
  }

  function setMuted(next) {
    muted = next;
    localStorage.setItem('forge.muted', muted ? '1' : '0');
    if (masterGain && audioCtx) {
      const t = audioCtx.currentTime;
      masterGain.gain.cancelScheduledValues(t);
      masterGain.gain.setValueAtTime(masterGain.gain.value, t);
      masterGain.gain.linearRampToValueAtTime(muted ? 0 : currentVolume, t + 0.08);
    }
    const el = $('btn-mute');
    if (el) {
      el.textContent = muted ? 'MUTED' : 'SOUND';
      el.classList.toggle('on', !muted);
    }
    return muted;
  }
  window.toggleMute = () => setMuted(!muted);
  if ($('btn-mute')) {
    $('btn-mute').addEventListener('click', () => setMuted(!muted));
    setMuted(muted); // paint the chip from the remembered state
  }

  const volSlider = $('theory-vol-slider');
  const volVal = $('theory-vol-val');
  if (volSlider) {
    volSlider.value = Math.round(currentVolume * 100);
    if (volVal) volVal.textContent = `${Math.round(currentVolume * 100)}%`;
    volSlider.addEventListener('input', (ev) => {
      const pct = parseInt(ev.target.value, 10);
      currentVolume = pct / 100.0;
      if (volVal) volVal.textContent = `${pct}%`;
      localStorage.setItem('forge.volume', String(currentVolume));
      if (masterGain && !muted && audioCtx) {
        const t = audioCtx.currentTime;
        masterGain.gain.cancelScheduledValues(t);
        masterGain.gain.setValueAtTime(masterGain.gain.value, t);
        masterGain.gain.linearRampToValueAtTime(currentVolume, t + 0.04);
      }
    });
  }

  function ringStarIdx(idx) {
    const s = spaceStars && spaceStars.find((x) => x.idx === idx);
    if (!s) return;
    audioBus();
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.frequency.value = s.milli_hz / 1000;
    osc.type = 'sine';
    const t0 = audioCtx.currentTime;
    gain.gain.setValueAtTime(0.0001, t0);
    gain.gain.exponentialRampToValueAtTime(0.09, t0 + 0.05);
    gain.gain.exponentialRampToValueAtTime(0.0001, t0 + 1.4);
    osc.connect(gain).connect(analyser);
    osc.start(t0);
    osc.stop(t0 + 1.45);
  }

  // Rust owns the SCORE (parse, lower, tick math — tested); this owns the
  // SOUND. One osc per note, scheduled against audioCtx.currentTime, which is
  // sample-accurate — firing them one IPC call at a time would only add jitter.
  let scorePlaying = false;
  async function playScore(name, repeats) {
    if (scorePlaying) return 0;
    const plan = await invokeCommand('score_plan', {
      name: name || 'contrapunctus',
      repeats: repeats || 3,
    });
    if (!plan || !plan.length) return 0;
    audioBus();
    if (audioCtx.state === 'suspended') await audioCtx.resume();
    scorePlaying = true;
    const t0 = audioCtx.currentTime + 0.12; // a beat of headroom to schedule in
    let last = 0;
    for (const n of plan) {
      const at = t0 + n.at_s;
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      osc.type = 'sine';
      osc.frequency.value = n.hz;
      const peak = Math.max(0.0001, n.gain * 0.18);
      gain.gain.setValueAtTime(0.0001, at);
      gain.gain.exponentialRampToValueAtTime(peak, at + 0.04);
      gain.gain.exponentialRampToValueAtTime(0.0001, at + n.dur_s);
      osc.connect(gain).connect(analyser);
      osc.start(at);
      osc.stop(at + n.dur_s + 0.05);
      last = Math.max(last, n.at_s + n.dur_s);
    }
    setTimeout(() => { scorePlaying = false; }, (last + 0.4) * 1000);
    const blurb = $('theory-blurb');
    if (blurb) blurb.textContent = `${plan.length} notes · ${last.toFixed(1)}s`;
    return plan.length;
  }
  window.playScore = playScore;

  // ── THEORY PANEL ────────────────────────────────────────────────────────
  // Drawn from the authored CATALOG that Rust ships, never hand-written here:
  // add a knob in theory.rs and it appears, with its own bounds and blurb.
  // Rust holds the values (TheoryStore), so the glass is a view, not a copy.
  let theoryAlchemical = false;

  async function playPlan(plan) {
    if (!plan || !plan.length) return 0;
    audioBus();
    if (audioCtx.state === 'suspended') await audioCtx.resume();
    const t0 = audioCtx.currentTime + 0.08;
    for (const n of plan) {
      const at = t0 + n.at_s;
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      osc.type = 'sine';
      osc.frequency.value = n.hz;
      const peak = Math.max(0.0001, n.gain * 0.08);
      gain.gain.setValueAtTime(0.0001, at);
      gain.gain.exponentialRampToValueAtTime(peak, at + 0.04);
      gain.gain.exponentialRampToValueAtTime(0.0001, at + n.dur_s);
      osc.connect(gain).connect(analyser);
      osc.start(at);
      osc.stop(at + n.dur_s + 0.05);
    }
    return plan.length;
  }

  function paintTheory(r) {
    if (!r) return;
    $('theory-key').textContent = `${r.root_name} · ${r.ms_per_beat} ms/beat`;
    $('theory-scale').textContent = r.scale_name.toUpperCase();
    $('theory-blurb').textContent = r.scale_blurb;
    $('theory-ref').textContent = `A${r.ref_a_hz.toFixed(0)}`;
    $('theory-notes').textContent = r.notes.join(' ');
    $('theory-beat').textContent = `${r.ms_per_beat} ms`;
    $('theory-tuning').textContent = r.ref_a_hz < 436 ? 'A440' : 'A432';
    theoryAlchemical = r.ref_a_hz < 436;

    $('theory-pulses').innerHTML = '';
    for (const on of r.pulses) {
      const d = document.createElement('span');
      d.className = on ? 'pulse on' : 'pulse';
      $('theory-pulses').appendChild(d);
    }

    const host = $('theory-knobs');
    host.innerHTML = '';
    for (const k of r.knobs) {
      const row = document.createElement('div');
      row.className = 'knob-row';
      row.title = k.blurb;
      const name = document.createElement('span');
      name.className = 'knob-label';
      name.textContent = k.label;
      row.appendChild(name);

      let input;
      if (k.kind === 'choice') {
        input = document.createElement('select');
        input.className = 'knob-select';
        k.choices.forEach((label, i) => {
          const o = document.createElement('option');
          o.value = String(k.min + i);
          o.textContent = label;
          input.appendChild(o);
        });
        input.value = String(k.value);
        input.addEventListener('change', async () => {
          paintTheory(await invokeCommand('theory_set', { id: k.id, value: Number(input.value) }));
        });
      } else {
        input = document.createElement('input');
        input.type = 'range';
        input.className = 'knob-slider';
        input.min = k.min; input.max = k.max; input.step = k.step; input.value = k.value;
        const out = document.createElement('b');
        out.className = 'knob-val';
        out.textContent = `${k.value}${k.unit ? ' ' + k.unit : ''}`;
        input.addEventListener('input', () => {
          out.textContent = `${input.value}${k.unit ? ' ' + k.unit : ''}`;
        });
        input.addEventListener('change', async () => {
          paintTheory(await invokeCommand('theory_set', { id: k.id, value: Number(input.value) }));
        });
        row.appendChild(input);
        row.appendChild(out);
        host.appendChild(row);
        continue;
      }
      row.appendChild(input);
      host.appendChild(row);
    }
  }

  async function openTheory() {
    const pane = $('theory-panel');
    pane.classList.toggle('hidden');
    if (!pane.classList.contains('hidden')) paintTheory(await invokeCommand('theory_read'));
  }
  if ($('btn-theory')) $('btn-theory').addEventListener('click', openTheory);
  if ($('theory-close')) $('theory-close').addEventListener('click', () => $('theory-panel').classList.add('hidden'));
  if ($('theory-audition')) $('theory-audition').addEventListener('click', async () => {
    playPlan(await invokeCommand('theory_audition'));
  });
  if ($('theory-play-bach')) $('theory-play-bach').addEventListener('click', () => playScore('contrapunctus'));
  if ($('theory-reset')) $('theory-reset').addEventListener('click', async () => {
    paintTheory(await invokeCommand('theory_reset'));
  });
  if ($('theory-tuning')) $('theory-tuning').addEventListener('click', async () => {
    paintTheory(await invokeCommand('theory_tuning', { alchemical: !theoryAlchemical }));
  });

  function selectStar(idx) {
    activeStarIdx = idx;
    const s = spaceStars && spaceStars.find((x) => x.idx === idx);
    if (!s) return;
    $('hud-name').textContent = s.name;
    $('hud-hz').textContent = `${(s.milli_hz / 1000).toFixed(2)} Hz`;
    $('hud-mag').textContent = s.mag_pmy;
    $('star-hud').classList.remove('hidden');

    // Update Floating Gemma Sky HUD
    const skyGemmaStatus = $('sky-gemma-status');
    if (skyGemmaStatus) {
      skyGemmaStatus.textContent = `Celestial Navigator: Steering toward ${s.name} · SO(5) Active (409.3 Gweights/s)`;
    }

    // Naturally link to constellation chain
    toggleConstellationNode(s);
  }

  // Crawl + strike: the void (not chrome/terminal) is live sky.
  const onVoid = (ev) => !ev.target.closest('.term-dock, .app-header, .status-pane, .star-hud, .rite-pane, .dossier, button, input, label');
  let voidDrag = false, voidPan = false, voidMoved = 0, voidX = 0, voidY = 0;
  window.addEventListener('mousedown', (ev) => {
    if (!onVoid(ev)) return;
    if (ev.button === 0) { voidDrag = true; voidMoved = 0; voidX = ev.clientX; voidY = ev.clientY; }
    if (ev.button === 2) { voidPan = true; voidMoved = 0; voidX = ev.clientX; voidY = ev.clientY; }
  });
  window.addEventListener('mouseup', () => { voidDrag = false; voidPan = false; });
  window.addEventListener('contextmenu', (ev) => { if (onVoid(ev)) ev.preventDefault(); });
  window.addEventListener('mousemove', (ev) => {
    const dx = ev.clientX - voidX, dy = ev.clientY - voidY;
    if (voidDrag) {
      voidMoved += Math.abs(dx) + Math.abs(dy);
      cam5d.yaw += dx * 0.006;
      cam5d.pitch = Math.max(-1.45, Math.min(1.45, cam5d.pitch + dy * 0.006));
      vel.yaw = dx * 0.003;
      vel.pitch = dy * 0.003;
    } else if (voidPan) {
      voidMoved += Math.abs(dx) + Math.abs(dy);
      const k = cam5d.distance * 0.0025;
      cam5d.tx -= dx * k * Math.cos(cam5d.yaw);
      cam5d.tz += dx * k * Math.sin(cam5d.yaw);
      cam5d.ty += dy * k;
    } else { return; }
    voidX = ev.clientX; voidY = ev.clientY;
    scheduleRefreshSky();
  });
  // Double-strike: a star becomes the focus.
  window.addEventListener('dblclick', (ev) => {
    if (!onVoid(ev) || voidMoved > 6 || !spaceStars) return;
    let best = null, bestD = 24 * 24;
    for (const s of spaceStars) {
      const d = (s._px - ev.clientX) ** 2 + (s._py - ev.clientY) ** 2;
      if (d < bestD) { bestD = d; best = s; }
    }
    let to = best ? [best.wx, best.wy, best.wz] : null;
    if (!to) {
      const deep = hygPick(ev.clientX, ev.clientY);
      if (deep >= 0 && glSky.pick) {
        // Sail toward a deep star: its dome direction, held at the roam wall.
        const b = deep * 8;
        const cl = Math.cos(glSky.lstRad), sl = Math.sin(glSky.lstRad);
        const x = glSky.pick[b] * cl + glSky.pick[b + 2] * sl;
        const z = -glSky.pick[b] * sl + glSky.pick[b + 2] * cl;
        const len = Math.hypot(x, glSky.pick[b + 1], z) || 1;
        to = [(x / len) * 140, (glSky.pick[b + 1] / len) * 140, (z / len) * 140];
        showDossier(deep, ev.clientX, ev.clientY);
      }
    }
    if (to) {
      flyTo = { from: [cam5d.tx, cam5d.ty, cam5d.tz], to, t0: performance.now(), dur: 650 };
      if (best) selectStar(best.idx);
    }
  });
  window.addEventListener('wheel', (ev) => {
    if (!onVoid(ev)) return;
    if (ev.ctrlKey) {
      cam5d.distance = Math.max(2.0, Math.min(600.0, cam5d.distance + ev.deltaY * 0.12));
    } else {
      flyTo = null;
      vel.fwd += ev.deltaY * 0.035;
    }
    scheduleRefreshSky();
  }, { passive: true });
  window.addEventListener('click', (ev) => {
    if (!onVoid(ev) || voidMoved > 6 || !spaceStars) return;
    let bestStar = null, bestD = 22 * 22;
    for (const s of spaceStars) {
      const d = (s._px - ev.clientX) ** 2 + (s._py - ev.clientY) ** 2;
      if (d < bestD) { bestD = d; bestStar = s; }
    }
    if (bestStar) {
      selectStar(bestStar.idx);
      ringStarIdx(bestStar.idx);
      return;
    }
    // No lore star under the strike — every deep star answers too.
    const deep = hygPick(ev.clientX, ev.clientY);
    if (deep >= 0) showDossier(deep, ev.clientX, ev.clientY);
    else if (!dossierPinned) $('star-dossier').classList.add('hidden');
  });

  const btnChainClear = $('btn-chain-clear');
  if (btnChainClear) {
    btnChainClear.addEventListener('click', (e) => {
      e.stopPropagation();
      clearConstellationChain();
    });
  }

  window.addEventListener('keydown', (ev) => {
    // Escape ALWAYS closes, pinned or not — the pin locks against stray
    // void-clicks, never against the operator asking for it to go away.
    if (ev.key === 'Escape') {
      closeDossier();
      $('theory-panel').classList.add('hidden');
      if (constellationChain.length > 0) clearConstellationChain();
    }
  });

  // ── THE FINGER OF GOD — today's receipted sky event, drawn on the sky.
  // Receipt (prokerala planet-aspects-august-2026): 2026-08-26 Sun 3°Vir
  // quincunx Pluto 3°Aqu (16:09) AND quincunx Neptune 3°Ari (20:30); the
  // Pluto-Neptune base is an exact sextile — a YOD, finger on the Sun.
  const SKY_EVENTS = [{
    date: '2026-08-26',
    name: 'YOD — ♇ ⚹ ♆ → ☉',
    reading: 'the finger of god falls on the sun',
    base: [{ glyph: '♇', lon: 303 }, { glyph: '♆', lon: 3 }],
    apex: { glyph: '☉', lon: 153 },
  }];
  const todayEvent = () => SKY_EVENTS.find((e) => e.date === new Date().toISOString().slice(0, 10)) || null;

  // Ecliptic longitude (β=0) -> equatorial direction (obliquity 23.4367°).
  function eclToDir(lonDeg) {
    const e = (23.4367 * Math.PI) / 180, l = (lonDeg * Math.PI) / 180;
    return { ra: Math.atan2(Math.sin(l) * Math.cos(e), Math.cos(l)), dec: Math.asin(Math.sin(e) * Math.sin(l)) };
  }

  // Project an absolute sky direction at the lore radius — same LST-rotate +
  // view_proj law as hygPick/STAR_VS, so the Yod rides the turning heaven.
  function projectSkyDir(ra, dec) {
    const g = glSky;
    if (!g.viewProj) return null;
    const R = 60;
    let x = R * Math.cos(dec) * Math.cos(ra), z = R * Math.cos(dec) * Math.sin(ra);
    const y = R * Math.sin(dec);
    const cl = Math.cos(g.lstRad), sl = Math.sin(g.lstRad);
    const rx = x * cl + z * sl, rz = -x * sl + z * cl;
    x = rx; z = rz;
    const m = g.viewProj;
    const w = m[3] * x + m[7] * y + m[11] * z + m[15];
    if (w <= 0) return null;
    return {
      x: (0.5 + 0.5 * ((m[0] * x + m[4] * y + m[8] * z + m[12]) / w)) * spaceCanvas.width,
      y: (0.5 - 0.5 * ((m[1] * x + m[5] * y + m[9] * z + m[13]) / w)) * spaceCanvas.height,
    };
  }

  function drawYod() {
    const ev = todayEvent();
    if (!ev) return;
    const at = (b) => { const d = eclToDir(b.lon); return { glyph: b.glyph, p: projectSkyDir(d.ra, d.dec) }; };
    const base = ev.base.map(at);
    const apex = at(ev.apex);
    if (!apex.p || base.some((b) => !b.p)) return;
    const ctx = spaceCtx;
    ctx.save();
    ctx.globalAlpha = 0.75;
    ctx.strokeStyle = '#C8791E';
    ctx.fillStyle = '#C8791E';
    ctx.lineWidth = 1;
    ctx.setLineDash([5, 4]);
    ctx.beginPath(); ctx.moveTo(base[0].p.x, base[0].p.y); ctx.lineTo(base[1].p.x, base[1].p.y); ctx.stroke();
    ctx.setLineDash([]);
    for (const b of base) {
      ctx.beginPath(); ctx.moveTo(b.p.x, b.p.y); ctx.lineTo(apex.p.x, apex.p.y); ctx.stroke();
    }
    // The finger: an arrowhead where the two quincunx rays converge.
    const mx = (base[0].p.x + base[1].p.x) / 2, my = (base[0].p.y + base[1].p.y) / 2;
    const al = Math.hypot(apex.p.x - mx, apex.p.y - my) || 1;
    const ux = (apex.p.x - mx) / al, uy = (apex.p.y - my) / al;
    ctx.beginPath();
    ctx.moveTo(apex.p.x, apex.p.y);
    ctx.lineTo(apex.p.x - ux * 12 - uy * 5, apex.p.y - uy * 12 + ux * 5);
    ctx.lineTo(apex.p.x - ux * 12 + uy * 5, apex.p.y - uy * 12 - ux * 5);
    ctx.closePath(); ctx.fill();
    ctx.font = '13px "Iosevka", "Cascadia Code", Consolas, monospace';
    ctx.textAlign = 'center';
    for (const b of base) ctx.fillText(b.glyph, b.p.x, b.p.y - 8);
    ctx.fillText(ev.apex.glyph, apex.p.x + ux * 16, apex.p.y + uy * 16 + 4);
    ctx.globalAlpha = 0.6;
    ctx.font = '10px "Iosevka", "Cascadia Code", Consolas, monospace';
    ctx.fillText(ev.name + ' — ' + ev.reading, apex.p.x, apex.p.y + uy * 16 + 20);
    ctx.restore();
  }

  // ── THE STAR DOSSIER — strike ANY of the 119k stars and a high-contrast
  // glass names it: class, temperature, distance, position, and the world
  // seed its mesh-world will grow from (WorldBuilderEngine lane, forge-core
  // zones::worldbuilder — descent is the named next door).
  function hygPick(mx, my) {
    const g = glSky;
    if (!g.ready || !g.viewProj || !g.pick) return -1;
    const m = g.viewProj, w = spaceCanvas.width, h = spaceCanvas.height;
    const cl = Math.cos(g.lstRad), sl = Math.sin(g.lstRad);
    const gx = cam5d.tx - g.eye[0], gy = cam5d.ty - g.eye[1], gz = cam5d.tz - g.eye[2];
    const glen = Math.hypot(gx, gy, gz) || 1.0;
    const gazeVec = [gx / glen, gy / glen, gz / glen];
    const v = g.pick;
    let best = -1, bestScore = 1e9;
    for (let i = 0; i < g.starCount; i++) {
      const b = i * 8;
      const isSol = v[b + 7] < 0;
      const p5d = transformStar5d(
        v[b], v[b + 1], v[b + 2], v[b + 6], v[b + 7], isSol,
        g.eye, gazeVec, cl, sl, so5State.theta_zw, so5State.phi_wv, so5State.beta_lorentz
      );
      const cw = m[3] * p5d.x + m[7] * p5d.y + m[11] * p5d.z + m[15];
      if (cw <= 0) continue;
      const px = (0.5 + 0.5 * (m[0] * p5d.x + m[4] * p5d.y + m[8] * p5d.z + m[12]) / cw) * w;
      const py = (0.5 - 0.5 * (m[1] * p5d.x + m[5] * p5d.y + m[9] * p5d.z + m[13]) / cw) * h;
      const d2 = (px - mx) ** 2 + (py - my) ** 2;
      if (d2 < 225) {
        const score = d2 - v[b + 6] * 160; // brightness breaks ties
        if (score < bestScore) { bestScore = score; best = i; }
      }
    }
    return best;
  }

  // ── FORGE-COLOUR & OKLCH / ANSI ENGINE ──
  function srgbToLinear(c) {
    const v = c / 255;
    return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  }
  function linearToSrgb(v) {
    const c = v <= 0.0031308 ? 12.92 * v : 1.055 * Math.pow(Math.max(0, v), 1.0 / 2.4) - 0.055;
    return Math.max(0, Math.min(255, Math.round(c * 255)));
  }
  function rgbToOklch(r, g, b) {
    const lr = srgbToLinear(r), lg = srgbToLinear(g), lb = srgbToLinear(b);
    const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
    const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
    const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
    const L = 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s;
    const a = 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s;
    const bVal = 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s;
    const C = Math.hypot(a, bVal);
    let H = (Math.atan2(bVal, a) * 180) / Math.PI;
    if (H < 0) H += 360;
    return { L, C, H };
  }
  function oklchToRgb(L, C, Hdeg) {
    const hRad = (Hdeg * Math.PI) / 180;
    const a = C * Math.cos(hRad), bVal = C * Math.sin(hRad);
    const l = L + 0.3963377774 * a + 0.2158037573 * bVal;
    const m = L - 0.1055613458 * a - 0.0638541728 * bVal;
    const s = L - 0.0894841775 * a - 1.2914855480 * bVal;
    const l3 = l * l * l, m3 = m * m * m, s3 = s * s * s;
    const lr = +4.0767434770 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3;
    const lg = -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3;
    const lb = -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3;
    return [linearToSrgb(lr), linearToSrgb(lg), linearToSrgb(lb)];
  }

  const SPECTRAL_BANDS = [[30000, 'O'], [10000, 'B'], [7500, 'A'], [6000, 'F'], [5200, 'G'], [3700, 'K'], [0, 'M']];
  function spectralOf(kelvin) {
    for (const [lo, cls] of SPECTRAL_BANDS) { if (kelvin >= lo) return cls; }
    return 'M';
  }
  // FNV-1a over the star's identity — the deterministic world seed its
  // mesh-world grows from (same-seed = same-world law).
  function worldSeedOf(i) {
    let hsh = 0x811c9dc5;
    const mix = (v) => { hsh ^= v & 0xff; hsh = Math.imul(hsh, 0x01000193); };
    mix(i); mix(i >> 8); mix(i >> 16); mix(0x13); mix(0xF0);
    return (hsh >>> 0).toString(16).padStart(8, '0');
  }

  // The lore stars' positions in BAKE/GL index space — the space hygPick and
  // the meta arrays speak. Scanned once; meta.lore is 255 for the multitude.
  let loreGl = null;
  function loreStarIndices() {
    if (!loreGl) {
      loreGl = [];
      const meta = glSky.meta;
      if (meta && meta.lore) {
        for (let k = 0; k < glSky.starCount; k++) {
          if (meta.lore[k] !== 255) loreGl.push(k);
        }
      }
    }
    return loreGl;
  }

  let loreNames = null;
  async function loreNamesOnce() {
    if (!loreNames) loreNames = (await invokeCommand('get_sky_chart')) || [];
    return loreNames;
  }

  // Spectral class bands (upper-K, letter) for badge + OBAFGKM strip marker.
  const CLASS_BANDS = { O: [30000, 40000], B: [10000, 30000], A: [7500, 10000], F: [6000, 7500], G: [5200, 6000], K: [3700, 5200], M: [2000, 3700] };

  // Live DSP oscilloscope: the SAME AnalyserNode the ring oscillators feed —
  // the scope draws the waveform actually sounding, never a fake trace.
  let scopeRaf = 0;
  function scopeLoop() {
    const cv = $('dossier-scope');
    const c2 = cv.getContext('2d');
    c2.clearRect(0, 0, cv.width, cv.height);
    if (analyser) {
      const data = new Uint8Array(analyser.fftSize);
      analyser.getByteTimeDomainData(data);
      c2.strokeStyle = 'rgba(255, 213, 74, 0.9)';
      c2.lineWidth = 1;
      c2.beginPath();
      for (let x = 0; x < cv.width; x++) {
        const v = data[Math.floor((x / cv.width) * data.length)] / 255;
        const y = 2 + v * (cv.height - 4);
        if (x === 0) c2.moveTo(x, y); else c2.lineTo(x, y);
      }
      c2.stroke();
    }
    if (!$('star-dossier').classList.contains('hidden')) scopeRaf = requestAnimationFrame(scopeLoop);
  }

  async function showDossier(i, atX, atY) {
    // A pinned card HOLDS. It used to be overwritten by the next star click,
    // which made the pin do nothing but keep an ever-changing card on screen.
    if (dossierPinned && !$('star-dossier').classList.contains('hidden')) return;
    const g = glSky, meta = g.meta;
    const pane = $('star-dossier');
    const isSol = i === g.starCount - 1;
    const kelvin = Math.round(2000 + (meta.teff[i] / 255) * 38000);
    const cls = spectralOf(kelvin);
    const [lo, hi] = CLASS_BANDS[cls];
    const sub = Math.min(9, Math.max(0, Math.floor(((hi - kelvin) / (hi - lo)) * 10)));
    // RECORDED identity, not a label: the bake's designation section carries
    // this star's own catalogue name (proper / Bayer / HIP / HD / Gliese).
    let name = isSol ? 'SOL' : `HYG ${i}`;
    if (!isSol) {
      const desig = await invokeCommand('star_designation', { idx: i });
      if (desig) name = desig.toUpperCase();
      if (meta.lore[i] !== 255) {
        const rows = await loreNamesOnce();
        const lore = rows[meta.lore[i]];
        if (lore) name = lore.name.toUpperCase();
      }
    }
    // This star's own voice, asked for in BAKE index space. spaceStars is NOT
    // that space — it numbers only the mag<=6.5 subset — so looking `i` up in
    // it found nothing and this readout went blank.
    const voiceMhz = isSol ? 0 : await invokeCommand('star_voice', { idx: i });
    // Consort: the nearest of the 16 lore stars by true angular distance.
    // Found through meta.lore, not through a name field — get_starmap_5d
    // stamps name: "" on every star, so a name test matches nothing.
    let consortGl = -1;
    if (!isSol) {
      const dx = Math.cos(meta.dec[i]) * Math.cos(meta.ra[i]);
      const dy = Math.sin(meta.dec[i]);
      const dz = Math.cos(meta.dec[i]) * Math.sin(meta.ra[i]);
      let bestDot = -2;
      for (const k of loreStarIndices()) {
        if (k === i) continue;
        const ex = Math.cos(meta.dec[k]) * Math.cos(meta.ra[k]);
        const ey = Math.sin(meta.dec[k]);
        const ez = Math.cos(meta.dec[k]) * Math.sin(meta.ra[k]);
        const dot = ex * dx + ey * dy + ez * dz;
        if (dot > bestDot) { bestDot = dot; consortGl = k; }
      }
    }
    const consortLore = consortGl >= 0 ? meta.lore[consortGl] : 255;
    const loreRows = consortLore !== 255 ? await loreNamesOnce() : null;
    const consortName = loreRows && loreRows[consortLore] ? loreRows[consortLore].name : null;
    const raH = ((meta.ra[i] / (Math.PI * 2)) * 24 + 24) % 24;
    const decD = (meta.dec[i] * 180) / Math.PI;
    const distPc = meta.dist[i];

    // Relativistic Doppler & 5D / OKLCH / ANSI TrueColor Telemetry
    let dopplerFactor = 1.0;
    let oklchStr = '—';
    let ansiStr = '—';
    let dopplerStr = '1.00×';
    if (glSky.viewProj && glSky.eye) {
      const sx = meta.x ? meta.x[i] : Math.cos(meta.dec[i]) * Math.cos(meta.ra[i]) * 60;
      const sy = meta.y ? meta.y[i] : Math.sin(meta.dec[i]) * 60;
      const sz = meta.z ? meta.z[i] : Math.cos(meta.dec[i]) * Math.sin(meta.ra[i]) * 60;
      const brightNorm = Math.max(0, Math.min(1, 1.0 - (meta.mag[i] || 0) / 6.5));
      const gx = cam5d.tx - glSky.eye[0], gy = cam5d.ty - glSky.eye[1], gz = cam5d.tz - glSky.eye[2];
      const glen = Math.hypot(gx, gy, gz) || 1.0;
      const gazeVec = [gx / glen, gy / glen, gz / glen];
      const cl = Math.cos(glSky.lstRad), sl = Math.sin(glSky.lstRad);
      const p5d = transformStar5d(
        sx, sy, sz, brightNorm, (i % 7) * 0.9, isSol,
        glSky.eye, gazeVec, cl, sl, so5State.theta_zw, so5State.phi_wv, so5State.beta_lorentz
      );
      dopplerFactor = p5d.doppler || 1.0;
      dopplerStr = `${dopplerFactor.toFixed(2)}× ${dopplerFactor > 1.05 ? '▲BLUE' : dopplerFactor < 0.95 ? '▼RED' : 'REST'}`;

      const tNorm = Math.max(0, Math.min(1, (kelvin - 2000) / 38000));
      const rWarm = Math.max(0.1, 1.0 - tNorm * 0.7);
      const gWarm = Math.max(0.1, 0.4 + tNorm * 0.55);
      const bCool = Math.max(0.1, 0.2 + tNorm * 0.8);
      const baseOklch = rgbToOklch(Math.round(rWarm * 255), Math.round(gWarm * 255), Math.round(bCool * 255));
      
      let hMod = (baseOklch.H + (so5State.phi_wv * 180 / Math.PI)) % 360;
      if (hMod < 0) hMod += 360;
      let lMod = Math.min(0.98, Math.max(0.05, baseOklch.L * Math.pow(dopplerFactor, 0.25)));
      let cMod = Math.min(0.35, Math.max(0.01, baseOklch.C * Math.pow(dopplerFactor, 0.5)));
      
      oklchStr = `L:${lMod.toFixed(2)} C:${cMod.toFixed(2)} H:${Math.round(hMod)}°`;
      const [ar, ag, ab] = oklchToRgb(lMod, cMod, hMod);
      ansiStr = `\\x1b[38;2;${ar};${ag};${ab}m`;
    }

    $('dossier-cat').textContent = isSol ? 'THE ORBIT HEART' : `LODE ${i.toString(36).toUpperCase()} · ${kelvin.toLocaleString()} K`;
    $('dossier-name').textContent = name;
    $('dossier-spectral').textContent = `${cls}${sub}${isSol ? 'V' : ''}`;
    $('dossier-hz').textContent = isSol ? '—' : voiceMhz ? `${(voiceMhz / 1000).toFixed(2)} Hz` : '—';
    $('dossier-key').textContent = isSol
      ? '—'
      : `${starBlinkHz(meta.lore[i] !== 255 ? meta.lore[i] : i).toFixed(3)} Hz`;
    $('dossier-mag').textContent = meta.mag[i].toFixed(2);
    if ($('dossier-oklch')) $('dossier-oklch').textContent = oklchStr;
    if ($('dossier-ansi')) {
      $('dossier-ansi').textContent = ansiStr;
      $('dossier-ansi').title = `ANSI TrueColor escape code: ${ansiStr}`;
    }
    if ($('dossier-doppler')) $('dossier-doppler').textContent = dopplerStr;
    $('dossier-dist').textContent = isSol ? '1 au' : distPc ? `${distPc} pc · ${Math.round(distPc * 3.262)} ly` : 'unmeasured';
    $('dossier-pos').textContent = isSol ? '0 · 0 · 0' : `${Math.floor(raH)}h${Math.round((raH % 1) * 60)}m ${decD >= 0 ? '+' : ''}${decD.toFixed(0)}°`;
    $('dossier-spec-marker').style.left = `${Math.min(Math.max((1 - (kelvin - 2000) / 38000) * 100, 0), 100)}%`;
    $('dossier-foot').textContent = `WORLDBUILDER · SEED 0x${worldSeedOf(i)} — a mesh world sleeps here`;

    // ── WORLDBUILDER BLUEPRINT & MESH LEDGER (PaTeX 5D + 6-Validator Gate) ──
    // The world belongs to the CLICKED star: the lore anchor only picks the
    // blueprint archetype, the seed is this star's own worldSeedOf.
    const targetStarIdx = isSol ? 0 : consortLore !== 255 ? consortLore : (i % 16);
    const worldSeed = isSol ? null : parseInt(worldSeedOf(i), 16);
    try {
      const world = await invokeCommand('generate_star_world', { starIdx: targetStarIdx, customSeed: worldSeed });
      if (world) {
        currentDossierWorld = world;
        if ($('wb-val-pill')) $('wb-val-pill').textContent = `${world.validation_score.toLocaleString()} PMY · ${world.validation_status.toUpperCase()}`;
        if ($('wb-node-info')) $('wb-node-info').textContent = `${world.room_count} NODES`;
        if ($('wb-depth-info')) $('wb-depth-info').textContent = `DEPTH ${world.ledger_depth}`;
        if ($('wb-svg-view')) $('wb-svg-view').innerHTML = world.svg_markup;
        const slider = $('wb-scrub-range');
        if (slider) {
          slider.max = world.ledger_depth;
          slider.value = world.ledger_depth;
        }
        if ($('wb-scrub-cur')) $('wb-scrub-cur').textContent = world.ledger_depth;
      }
    } catch (e) {
      console.warn('generate_star_world failed:', e);
    }

    pane.classList.remove('hidden');
    cancelAnimationFrame(scopeRaf);
    scopeRaf = requestAnimationFrame(scopeLoop);
  }

  // Worldbuilder ledger scrubber event listener
  let currentDossierWorld = null;
  let scrubDebounce = 0;
  if ($('wb-scrub-range')) {
    $('wb-scrub-range').addEventListener('input', (ev) => {
      const depth = parseInt(ev.target.value, 10);
      if ($('wb-scrub-cur')) $('wb-scrub-cur').textContent = depth;
      if (!currentDossierWorld) return;
      clearTimeout(scrubDebounce);
      scrubDebounce = setTimeout(async () => {
        try {
          const replayed = await invokeCommand('replay_world_ledger', {
            starIdx: currentDossierWorld.star_idx,
            seedHex: currentDossierWorld.seed_hex,
            depth: depth
          });
          if (replayed) {
            if ($('wb-svg-view')) $('wb-svg-view').innerHTML = replayed.svg_markup;
            if ($('wb-depth-info')) $('wb-depth-info').textContent = `DEPTH ${replayed.ledger_depth}`;
          }
        } catch (e) {
          console.warn('replay_world_ledger failed:', e);
        }
      }, 50);
    });
  }

  // ── THE ASTROLABE WITHIN — scroll INTO the heart of the sky and the brass
  // instrument rises (forge_core_v3::astrolabe via the still-landed
  // get_astrolabe_state command; monochromatic bronze, old-master ground).
  let astroState = null, astroRotFetched = -1e9, astroBusy = false;
  async function fetchAstro(rotCdeg) {
    if (astroBusy) return;
    astroBusy = true;
    const s = await invokeCommand('get_astrolabe_state', {
      reteRotCdeg: rotCdeg, alidadeCdeg: 4500,
      activeStarIdx: activeStarIdx >= 0 ? activeStarIdx % 16 : 0,
    });
    astroBusy = false;
    if (s) { astroState = s; astroRotFetched = rotCdeg; }
  }
  // The slider is a FLOOR: flight-into-Sol still raises the brass by itself.
  let astroManual = 0;
  $('astro-slider').addEventListener('input', (ev) => { astroManual = ev.target.value / 100; });
  function astroImmersion() {
    const e = glSky.eye;
    const d = Math.hypot(e[0], e[1], e[2]);
    return Math.max(Math.min(Math.max((46 - d) / 22, 0), 1), astroManual);
  }
  const BRONZE = '#C8791E', BRONZE_DIM = '#B08A63', BRONZE_HOT = '#FF6A1A';
  function drawAstrolabe(a) {
    const rot = Math.round((((skyLstDeg * 100) % 36000) + 36000) % 36000);
    if (Math.abs(rot - astroRotFetched) > 25) fetchAstro(rot);
    if (!astroState) return;
    const w = spaceCanvas.width, h = spaceCanvas.height;
    const cx = w / 2, cy = h / 2;
    const R = Math.min(w, h) * (0.30 + 0.10 * a);
    const ctx = spaceCtx;
    ctx.save();
    ctx.globalAlpha = a;
    ctx.lineWidth = 1.2;
    ctx.strokeStyle = BRONZE;
    ctx.shadowColor = BRONZE;
    ctx.shadowBlur = 8 * a;
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, Math.PI * 2); ctx.stroke();
    ctx.shadowBlur = 0;
    ctx.strokeStyle = BRONZE_DIM;
    ctx.beginPath(); ctx.arc(cx, cy, R * 0.97, 0, Math.PI * 2); ctx.stroke();
    for (let i = 0; i < 72; i++) {
      const t = (i / 72) * Math.PI * 2;
      const l = i % 6 === 0 ? 0.935 : 0.955;
      ctx.beginPath();
      ctx.moveTo(cx + Math.cos(t) * R * l, cy + Math.sin(t) * R * l);
      ctx.lineTo(cx + Math.cos(t) * R * 0.97, cy + Math.sin(t) * R * 0.97);
      ctx.stroke();
    }
    for (const k of [0.63, 0.79, 0.92]) {
      ctx.beginPath(); ctx.arc(cx, cy, R * k, 0, Math.PI * 2); ctx.stroke();
    }
    const rr = (astroState.rete_rot_cdeg / 36000) * Math.PI * 2;
    ctx.save();
    ctx.translate(cx, cy);
    ctx.rotate(rr);
    ctx.strokeStyle = BRONZE;
    ctx.beginPath(); ctx.arc(0, -R * 0.145, R * 0.62, 0, Math.PI * 2); ctx.stroke();
    for (const s of astroState.stars) {
      const px = (s.x_pmy / 10000) * R, py = (s.y_pmy / 10000) * R;
      const hot = s.idx === astroState.active_star_idx;
      ctx.strokeStyle = hot ? BRONZE_HOT : BRONZE;
      ctx.fillStyle = hot ? BRONZE_HOT : BRONZE;
      ctx.shadowColor = ctx.fillStyle;
      ctx.shadowBlur = hot ? 10 : 4;
      ctx.beginPath(); ctx.moveTo(px * 0.82, py * 0.82); ctx.lineTo(px, py); ctx.stroke();
      ctx.beginPath(); ctx.arc(px, py, hot ? 3.2 : 2.1, 0, Math.PI * 2); ctx.fill();
      ctx.shadowBlur = 0;
    }
    ctx.restore();
    const al = (astroState.alidade_cdeg / 36000) * Math.PI * 2;
    ctx.strokeStyle = BRONZE_DIM;
    ctx.beginPath();
    ctx.moveTo(cx - Math.cos(al) * R, cy + Math.sin(al) * R);
    ctx.lineTo(cx + Math.cos(al) * R, cy - Math.sin(al) * R);
    ctx.stroke();
    const activeName = astroState.stars[astroState.active_star_idx] ? astroState.stars[astroState.active_star_idx].name : '';
    ctx.fillStyle = BRONZE_DIM;
    ctx.font = '11px "Iosevka", "Cascadia Code", Consolas, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(`THE ASTROLABE — alidade ${(astroState.altitude_cdeg / 100).toFixed(0)}° — ${activeName}`, cx, cy + R + 18);
    ctx.restore();
  }

  // ── Movable glass: drag any star info box where you want it; the pin
  // locks the dossier open against void-clicks and Esc.
  let dossierPinned = false;
  $('dossier-pin').addEventListener('click', () => {
    dossierPinned = !dossierPinned;
    $('dossier-pin').classList.toggle('on', dossierPinned);
  });
  // STOP: close the card and drop the pin with it, so a pinned card can never
  // strand itself open with no way back.
  function closeDossier() {
    dossierPinned = false;
    $('dossier-pin').classList.remove('on');
    $('star-dossier').classList.add('hidden');
    cancelAnimationFrame(scopeRaf);
  }
  window.closeDossier = closeDossier;
  if ($('dossier-close')) $('dossier-close').addEventListener('click', closeDossier);
  // The theory glass drags like the star glass — same law, same handle rules.
  if ($('theory-panel')) makeDraggable($('theory-panel'));

  function makeDraggable(el) {
    let drag = false, sx = 0, sy = 0, ox = 0, oy = 0;
    el.addEventListener('mousedown', (e) => {
      if (e.target.closest('input, button, canvas, .dossier-pin')) return;
      drag = true; sx = e.clientX; sy = e.clientY;
      const r = el.getBoundingClientRect();
      ox = r.left; oy = r.top;
      e.preventDefault();
      e.stopPropagation();
    });
    window.addEventListener('mousemove', (e) => {
      if (!drag) return;
      el.style.left = `${ox + e.clientX - sx}px`;
      el.style.top = `${oy + e.clientY - sy}px`;
      el.style.right = 'auto';
      el.dataset.moved = '1';
    });
    window.addEventListener('mouseup', () => { drag = false; });
  }
  makeDraggable($('star-dossier'));
  makeDraggable($('star-hud'));

  // ── Terminal dock (ConPTY → VT500 grid frames from the backend) ──
  const termCanvas = $('term-canvas');
  const termCtx = termCanvas.getContext('2d');
  const CW = 8.4, CH = 17;
  let termCols = 140, termRows = 18;
  let termBooted = false;

  const cellColor = (u32) =>
    `rgb(${(u32 >>> 24) & 255}, ${(u32 >>> 16) & 255}, ${(u32 >>> 8) & 255})`;

  // OKLCH ink grade (the sky_verb mag_ink look, applied to EVERY glyph): the
  // cell colour crosses to OKLCH, its lightness floor is lifted so dim ink
  // still reads and bright ink glows, hue and chroma ride untouched.
  const gradeCache = new Map();
  function okGrade(r, g, b) {
    const lin = (v) => { v /= 255; return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
    const unlin = (v) => { v = Math.max(0, Math.min(1, v)); return v <= 0.0031308 ? v * 12.92 : 1.055 * Math.pow(v, 1 / 2.4) - 0.055; };
    const lr = lin(r), lg = lin(g), lb = lin(b);
    let l_ = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
    let m_ = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
    let s_ = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
    let L = 0.2104542553 * l_ + 0.793617785 * m_ - 0.0040720468 * s_;
    const a = 1.9779984951 * l_ - 2.428592205 * m_ + 0.4505937099 * s_;
    const bb = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.808675766 * s_;
    L = Math.min(1, 0.3 + 0.72 * L);
    l_ = L + 0.3963377774 * a + 0.2158037573 * bb;
    m_ = L - 0.1055613458 * a - 0.0638541728 * bb;
    s_ = L - 0.0894841775 * a - 1.291485548 * bb;
    l_ **= 3; m_ **= 3; s_ **= 3;
    const R = unlin(4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_);
    const G = unlin(-1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_);
    const B = unlin(-0.0041960863 * l_ - 0.7034186147 * m_ + 1.707614701 * s_);
    return [Math.round(R * 255), Math.round(G * 255), Math.round(B * 255), L];
  }
  function inkOf(u32) {
    let hit = gradeCache.get(u32);
    if (!hit) {
      const [r, g, b, L] = okGrade((u32 >>> 24) & 255, (u32 >>> 16) & 255, (u32 >>> 8) & 255);
      hit = { css: `rgb(${r}, ${g}, ${b})`, glow: L };
      gradeCache.set(u32, hit);
    }
    return hit;
  }

  // Glass tokens (glass-terminal.tokens.vixi): the soot scrim is the canvas
  // element's own CSS background (continuous, slides with the accordion —
  // Sean 2026-08-26 "not gapped"); pane tint rides the bitmap; default VT
  // ground is keyed out so stars still ghost through; TUI cells stay opaque.
  const GLASS_TINT = 'rgba(158, 200, 225, 0.08)';
  const TERM_BG_DEFAULT = 0x0C0A08;

  // Selection + scrollback glass state. Selection rows are ABSOLUTE
  // scrollback indices (depth - view + screenRow, the one mapping
  // vt.rs:454 visible_cell resolves), so a highlight rides its text.
  let termLastFrame = null;
  let termSel = null;          // { a:{c,r}, b:{c,r} } or null, r absolute
  let termSelecting = false;
  let termAutoScroll = 0;

  function termAbsBase(f) {
    return f ? (f.depth | 0) - (f.view | 0) : 0;
  }

  function termRowText(f, y) {
    let s = '';
    for (const [text] of f.grid[y]) s += text;
    return s;
  }

  function termSelBounds() {
    if (!termSel) return null;
    const { a, b } = termSel;
    const fwd = a.r < b.r || (a.r === b.r && a.c <= b.c);
    return fwd ? { r1: a.r, c1: a.c, r2: b.r, c2: b.c } : { r1: b.r, c1: b.c, r2: a.r, c2: a.c };
  }

  function termSelectedText() {
    const f = termLastFrame, s = termSelBounds();
    if (!f || !s) return '';
    const base = termAbsBase(f);
    const lines = [];
    const y1 = Math.max(0, s.r1 - base), y2 = Math.min(f.grid.length - 1, s.r2 - base);
    for (let y = y1; y <= y2; y++) {
      const row = termRowText(f, y);
      const from = y + base === s.r1 ? s.c1 : 0;
      const to = y + base === s.r2 ? s.c2 + 1 : f.cols;
      lines.push(row.slice(from, to).replace(/\s+$/, ''));
    }
    return lines.join('\n');
  }

  function drawTermFrame(f) {
    termLastFrame = f;
    // Device-pixel backing store: size to actual element client box * DPR
    // so the bitmap is 1:1 with screen pixels and never gets stretched by CSS.
    const dpr = window.devicePixelRatio || 1;
    const clientW = termCanvas.clientWidth || Math.ceil(f.cols * CW);
    const clientH = termCanvas.clientHeight || (f.rows * CH);
    const needW = Math.ceil(clientW * dpr), needH = Math.ceil(clientH * dpr);
    if (termCanvas.width !== needW) termCanvas.width = needW;
    if (termCanvas.height !== needH) termCanvas.height = needH;
    termCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
    termCtx.clearRect(0, 0, clientW, clientH);
    termCtx.fillStyle = GLASS_TINT;
    termCtx.fillRect(0, 0, clientW, clientH);
    termCtx.font = '13px "Iosevka", "Cascadia Code", Consolas, monospace';
    termCtx.textBaseline = 'top';
    for (let y = 0; y < f.grid.length; y++) {
      let x = 0;
      for (const [text, fg, bg] of f.grid[y]) {
        if (((bg >>> 8) & 0xFFFFFF) !== TERM_BG_DEFAULT) {
          termCtx.fillStyle = cellColor(bg);
          termCtx.fillRect(x * CW, y * CH, text.length * CW, CH);
        }
        const ink = inkOf(fg >>> 0);
        termCtx.fillStyle = ink.css;
        termCtx.shadowColor = ink.css;
        termCtx.shadowBlur = ink.glow > 0.85 ? 6 : ink.glow > 0.65 ? 3 : 0;
        for (let i = 0; i < text.length; i++) {
          const ch = text[i];
          if (ch !== ' ') termCtx.fillText(ch, (x + i) * CW, y * CH + 2);
        }
        termCtx.shadowBlur = 0;
        x += text.length;
      }
    }
    // Cursor block (molten core, translucent so the glyph reads through).
    termCtx.fillStyle = 'rgba(255, 106, 26, 0.45)';
    termCtx.fillRect(f.cursor[0] * CW, f.cursor[1] * CH, CW, CH);
    // Selection veil (struck-bone tint over the selected cells).
    const sb = termSelBounds();
    if (sb) {
      const base = termAbsBase(f);
      termCtx.fillStyle = 'rgba(247, 233, 210, 0.22)';
      const y1 = Math.max(0, sb.r1 - base), y2 = Math.min(f.rows - 1, sb.r2 - base);
      for (let y = y1; y <= y2; y++) {
        const from = y + base === sb.r1 ? sb.c1 : 0;
        const to = y + base === sb.r2 ? sb.c2 + 1 : f.cols;
        termCtx.fillRect(from * CW, y * CH, Math.max(0, to - from) * CW, CH);
      }
    }
    // Scroll chip: only while looking into history.
    if (f.view > 0) {
      const label = `SCROLL ${f.view}/${f.depth} — End: live`;
      termCtx.font = '11px Consolas, monospace';
      const w = termCtx.measureText(label).width + 14;
      termCtx.fillStyle = 'rgba(26, 15, 9, 0.85)';
      termCtx.fillRect(clientW - w - 8, 4, w, 18);
      termCtx.fillStyle = '#FFD54A';
      termCtx.fillText(label, clientW - w - 1, 8);
      termCtx.font = '13px "Iosevka", "Cascadia Code", Consolas, monospace';
    }
  }

  // Measure cell position directly from CW and CH without distortion.
  // Row comes back ABSOLUTE.
  function termCellAt(e) {
    const rect = termCanvas.getBoundingClientRect();
    const f = termLastFrame;
    const cols = f?.cols ?? termCols, rows = f?.rows ?? termRows;
    const c = Math.max(0, Math.min(cols - 1, Math.floor((e.clientX - rect.left) / CW)));
    const r = Math.max(0, Math.min(rows - 1, Math.floor((e.clientY - rect.top) / CH)));
    return { c, r: r + termAbsBase(f) };
  }

  // Word span under an absolute cell, for the double-click grab.
  function termWordRange(absR, c) {
    const f = termLastFrame;
    if (!f) return null;
    const y = absR - termAbsBase(f);
    if (y < 0 || y >= f.grid.length) return null;
    const row = termRowText(f, y);
    const word = (ch) => !!ch && !/[\s()[\]{}<>"'`,;:|]/.test(ch);
    if (!word(row[c])) return { c1: c, c2: c };
    let a = c, b = c;
    while (a > 0 && word(row[a - 1])) a--;
    while (b < f.cols - 1 && word(row[b + 1])) b++;
    return { c1: a, c2: b };
  }

  termCanvas.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    termCanvas.focus();
    termMenuHide();
    const p = termCellAt(e);
    const cols = termLastFrame?.cols ?? termCols;
    if (e.detail >= 3) {
      termSel = { a: { c: 0, r: p.r }, b: { c: cols - 1, r: p.r } };
      termSelecting = false;
    } else if (e.detail === 2) {
      const w = termWordRange(p.r, p.c);
      termSel = { a: { c: w ? w.c1 : p.c, r: p.r }, b: { c: w ? w.c2 : p.c, r: p.r } };
      termSelecting = false;
    } else {
      termSel = { a: p, b: { c: p.c, r: p.r } };
      termSelecting = true;
    }
    if (termLastFrame) drawTermFrame(termLastFrame);
  });
  // Tracked on the window, not the canvas: a drag that leaves the glass must
  // keep extending, and passing the top/bottom edge pages the scrollback.
  window.addEventListener('mousemove', (e) => {
    if (!termSelecting || !termSel) return;
    termSel.b = termCellAt(e);
    const rect = termCanvas.getBoundingClientRect();
    termAutoScroll = e.clientY < rect.top ? 1 : e.clientY > rect.bottom ? -1 : 0;
    if (termLastFrame) drawTermFrame(termLastFrame);
  });
  setInterval(() => {
    if (termSelecting && termAutoScroll) invokeCommand('term_scroll', { delta: termAutoScroll });
  }, 90);
  window.addEventListener('mouseup', () => {
    const wasDrag = termSelecting;
    termSelecting = false;
    termAutoScroll = 0;
    // A bare click is not a selection — keep Ctrl+C as SIGINT.
    if (wasDrag && termSel && termSel.a.c === termSel.b.c && termSel.a.r === termSel.b.r) {
      termSel = null;
      if (termLastFrame) drawTermFrame(termLastFrame);
    }
  });

  // A selection can reach outside the viewport (Select All spans the whole
  // buffer) and the frame carries only visible rows, so anything off-screen
  // is sliced out of term_dump, which is indexed by the same absolute row.
  async function termSelectedTextFull() {
    const f = termLastFrame, s = termSelBounds();
    if (!f || !s) return '';
    const base = termAbsBase(f);
    if (s.r1 >= base && s.r2 <= base + f.rows - 1) return termSelectedText();
    const all = await invokeCommand('term_dump');
    if (!Array.isArray(all) || !all.length) return termSelectedText();
    const lines = [];
    const r2 = Math.min(s.r2, all.length - 1);
    for (let r = Math.max(0, s.r1); r <= r2; r++) {
      const row = all[r] ?? '';
      const from = r === s.r1 ? s.c1 : 0;
      const to = r === s.r2 ? s.c2 + 1 : f.cols;
      lines.push(row.slice(from, to).replace(/\s+$/, ''));
    }
    return lines.join('\n');
  }

  async function termCopySelection() {
    const text = await termSelectedTextFull();
    if (!text) return false;
    try { await navigator.clipboard.writeText(text); } catch (_) {}
    termSel = null;
    if (termLastFrame) drawTermFrame(termLastFrame);
    return true;
  }

  async function termPaste() {
    let text = '';
    try { text = await navigator.clipboard.readText(); } catch (_) { return; }
    if (text) invokeCommand('term_write', { data: text.replace(/\r\n/g, '\r') });
  }

  function termKeyBytes(e) {
    if (e.key.length === 1 && !e.ctrlKey && !e.altKey) return e.key;
    if (e.ctrlKey && e.key.length === 1) {
      const c = e.key.toUpperCase().charCodeAt(0);
      if (c >= 64 && c <= 95) return String.fromCharCode(c - 64);
    }
    switch (e.key) {
      case 'Enter': return '\r';
      case 'Backspace': return '\x7f';
      case 'Tab': return '\t';
      case 'Escape': return '\x1b';
      case 'ArrowUp': return '\x1b[A';
      case 'ArrowDown': return '\x1b[B';
      case 'ArrowRight': return '\x1b[C';
      case 'ArrowLeft': return '\x1b[D';
      case 'Home': return '\x1b[H';
      case 'End': return '\x1b[F';
      case 'Delete': return '\x1b[3~';
      default: return null;
    }
  }

  termCanvas.addEventListener('keydown', (e) => {
    // Clipboard lane first: Ctrl+Shift+C / Ctrl+Insert copy; Ctrl+C copies
    // ONLY with a live selection (else it stays SIGINT); Ctrl+V / Shift+Insert paste.
    const k = e.key;
    if ((e.ctrlKey && e.shiftKey && (k === 'C' || k === 'c')) || (e.ctrlKey && k === 'Insert')) {
      e.preventDefault(); termCopySelection(); return;
    }
    if (e.ctrlKey && !e.shiftKey && (k === 'c' || k === 'C') && termSel) {
      e.preventDefault(); termCopySelection(); return;
    }
    if ((e.ctrlKey && (k === 'v' || k === 'V')) || (e.shiftKey && k === 'Insert')) {
      e.preventDefault(); termPaste(); return;
    }
    // Scrollback paging: PageUp/PageDown page the history, End snaps to live.
    if (k === 'PageUp' || k === 'PageDown') {
      e.preventDefault();
      const page = Math.max(1, termRows - 1);
      invokeCommand('term_scroll', { delta: k === 'PageUp' ? page : -page });
      return;
    }
    if (e.ctrlKey && k === 'End') {
      e.preventDefault();
      invokeCommand('term_scroll', { delta: -1000000 });
      return;
    }
    const bytes = termKeyBytes(e);
    if (bytes !== null) {
      e.preventDefault();
      if (termSel) { termSel = null; if (termLastFrame) drawTermFrame(termLastFrame); }
      invokeCommand('term_write', { data: bytes });
    }
  });

  // Wheel = scrollback (backend viewport; refused while a TUI owns the alt screen).
  termCanvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    invokeCommand('term_scroll', { delta: e.deltaY < 0 ? 3 : -3 });
  }, { passive: false });

  // Right-click opens the glass menu. It used to paste on the spot (PuTTY
  // habit) — one stray click injected a whole clipboard into a live shell,
  // so paste is now a named item you aim at.
  const termMenu = document.createElement('div');
  termMenu.className = 'term-menu hidden';
  document.body.appendChild(termMenu);

  function termMenuHide() {
    termMenu.classList.add('hidden');
  }

  // The whole buffer, history included: absolute row 0 through the last live
  // grid row. Copy resolves the off-screen rows through term_dump.
  function termSelectAll() {
    const f = termLastFrame;
    if (!f) return;
    termSel = { a: { c: 0, r: 0 }, b: { c: f.cols - 1, r: (f.depth | 0) + f.rows - 1 } };
    drawTermFrame(f);
  }

  const TERM_MENU = [
    { label: 'Copy', hint: 'Ctrl+Shift+C', run: () => termCopySelection(), on: () => !!termSel },
    { label: 'Paste', hint: 'Ctrl+V', run: () => termPaste(), on: () => true },
    { label: 'Select All', hint: '', run: () => termSelectAll(), on: () => !!termLastFrame },
    { label: 'Clear Selection', hint: 'Esc', run: () => { termSel = null; if (termLastFrame) drawTermFrame(termLastFrame); }, on: () => !!termSel },
    { sep: true },
    { label: 'Scroll to Live', hint: 'Ctrl+End', run: () => invokeCommand('term_scroll', { delta: -1000000 }), on: () => ((termLastFrame?.view | 0) > 0) },
    { label: 'Clear Screen', hint: 'cls', run: () => invokeCommand('term_write', { data: 'cls\r' }), on: () => true },
  ];

  function termMenuShow(x, y) {
    termMenu.replaceChildren();
    for (const it of TERM_MENU) {
      if (it.sep) {
        const s = document.createElement('div');
        s.className = 'term-menu-sep';
        termMenu.appendChild(s);
        continue;
      }
      const row = document.createElement('button');
      row.className = 'term-menu-item';
      row.disabled = !it.on();
      const name = document.createElement('span');
      name.textContent = it.label;
      const hint = document.createElement('span');
      hint.className = 'term-menu-hint';
      hint.textContent = it.hint;
      row.append(name, hint);
      row.addEventListener('click', () => { termMenuHide(); it.run(); });
      termMenu.appendChild(row);
    }
    termMenu.classList.remove('hidden');
    termMenu.style.left = Math.min(x, window.innerWidth - termMenu.offsetWidth - 6) + 'px';
    termMenu.style.top = Math.min(y, window.innerHeight - termMenu.offsetHeight - 6) + 'px';
  }

  termCanvas.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    termMenuShow(e.clientX, e.clientY);
  });
  window.addEventListener('mousedown', (e) => {
    if (!termMenu.classList.contains('hidden') && !termMenu.contains(e.target)) termMenuHide();
  }, true);
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') termMenuHide();
  });

  // pwsh-style adjust: the CSS resize grip (and window resizes) re-deal the
  // ConPTY dims; rows/cols snap to whole cells.
  let resizeTimer = null;
  new ResizeObserver((entries) => {
    if (!termBooted) return;
    if (termLastFrame) {
      drawTermFrame(termLastFrame);
    }
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      const entry = entries[0];
      const width = entry ? entry.contentRect.width : termCanvas.clientWidth;
      const height = entry ? entry.contentRect.height : termCanvas.clientHeight;
      const cols = Math.max(20, Math.floor(width / CW));
      const rows = Math.max(4, Math.floor(height / CH));
      if (cols !== termCols || rows !== termRows) {
        termCols = cols; termRows = rows;
        invokeCommand('term_resize', { cols, rows });
      }
    }, 100);
  }).observe(termCanvas);

  // The dock's voice: the ear's phrase (term-notes, SungNote[]) sung as soft
  // sine swells on the same WebAudio lane as ringStarIdx. A short legato
  // queue with a hard backlog cap keeps a busy Gemini stream a melody, never
  // a wall of sound — the MELODY_MAX law, audible.
  let termSingT = 0;
  function termSing(notes) {
    if (!notes || !notes.length) return;
    if ($('term-dock').classList.contains('collapsed')) return;
    audioBus();
    const now = audioCtx.currentTime;
    termSingT = Math.max(termSingT, now);
    for (const n of notes) {
      if (termSingT - now > 0.8) break; // backlog cap: drop, never drone
      const hz = n.mhz / 1000;
      if (!(hz > 20)) continue;
      const dur = Math.min(0.5, Math.max(0.09, (n.ms | 0) / 1000));
      const t0 = termSingT;
      const osc = audioCtx.createOscillator();
      osc.type = 'sine';
      osc.frequency.value = hz;
      const shimmer = audioCtx.createOscillator();
      shimmer.type = 'sine';
      shimmer.frequency.value = hz * 2;
      const gain = audioCtx.createGain();
      const gain2 = audioCtx.createGain();
      // Schaeffer-style swell, held far under the star ring's 0.35 peak.
      gain.gain.setValueAtTime(0.0001, t0);
      gain.gain.exponentialRampToValueAtTime(0.05, t0 + 0.03);
      gain.gain.exponentialRampToValueAtTime(0.0001, t0 + dur * 2.2);
      gain2.gain.value = 0.18;
      osc.connect(gain).connect(analyser);
      shimmer.connect(gain2).connect(gain);
      osc.start(t0);
      shimmer.start(t0);
      osc.stop(t0 + dur * 2.3);
      shimmer.stop(t0 + dur * 2.3);
      termSingT += dur * 0.9; // legato: each note leans into the next
    }
  }

  async function termEnsureBoot() {
    if (termBooted) return;
    try {
      await listenEvent('term-grid', (ev) => drawTermFrame(ev.payload));
      await listenEvent('term-notes', (ev) => termSing(ev.payload));
      termCols = Math.max(20, Math.floor((termCanvas.clientWidth || 1180) / CW));
      termRows = Math.max(10, Math.floor((termCanvas.clientHeight || 420) / CH));
      const res = await invokeCommand('term_boot', { cols: termCols, rows: termRows });
      termBooted = res !== null;

      // Dynamically re-deal ConPTY buffer dimensions on any height/width change
      const ro = new ResizeObserver((entries) => {
        for (const entry of entries) {
          const w = entry.contentRect.width;
          const h = entry.contentRect.height;
          if (w > 50 && h > 50) {
            const cols = Math.max(20, Math.floor(w / CW));
            const rows = Math.max(8, Math.floor(h / CH));
            if (cols !== termCols || rows !== termRows) {
              termCols = cols;
              termRows = rows;
              invokeCommand('term_resize', { cols, rows });
            }
          }
        }
      });
      ro.observe(termCanvas);
    } catch (e) {
      console.error('[shell] termEnsureBoot failed:', e);
      termBooted = false;
    }
  }

  // ── SAND→GLASS emergence (ADR-0032 look, canvas parity; the GPU voxel-stencil
  // wire remains the native shell lane's later port).
  const emergeCanvas = $('emerge-canvas');
  const emergeCtx = emergeCanvas.getContext('2d');
  const SAND_LO = [126, 110, 72];   // sand.600
  const SAND_HI = [199, 176, 122];  // sand.300
  const GLASS_RGB = [158, 200, 225]; // glass.frost
  let emergeRunning = false;

  function runEmergence() {
    if (emergeRunning) return;
    emergeRunning = true;
    const dock = $('term-dock');
    emergeCanvas.width = dock.clientWidth;
    emergeCanvas.height = dock.clientHeight;
    emergeCanvas.style.display = 'block';
    let seed = 0xA032;
    const next = () => ((seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
    const grains = [];
    for (let i = 0; i < 140; i++) {
      grains.push({ x: next(), yT: next(), delay: next() * 0.45, mix: next(), r: 1 + next() * 2 });
    }
    const T = 1.2;
    let t0 = null;
    function step(ts) {
      if (t0 === null) t0 = ts;
      const t = (ts - t0) / 1000;
      const w = emergeCanvas.width, h = emergeCanvas.height;
      emergeCtx.clearRect(0, 0, w, h);
      if (t >= T) {
        emergeCanvas.style.display = 'none';
        emergeRunning = false;
        return;
      }
      const crystal = Math.max(0, (t - 0.85) / (T - 0.85));
      for (const g of grains) {
        const p = Math.min(1, Math.max(0, (t - g.delay) / 0.6));
        const ease = 1 - (1 - p) * (1 - p);
        const y = h - ease * (h * (0.15 + 0.85 * g.yT));
        const col = [0, 1, 2].map((i) => {
          const sand = SAND_LO[i] + (SAND_HI[i] - SAND_LO[i]) * g.mix;
          return Math.round(sand + (GLASS_RGB[i] - sand) * crystal);
        });
        emergeCtx.fillStyle = `rgba(${col[0]}, ${col[1]}, ${col[2]}, ${(0.85 * (1 - crystal * 0.8)).toFixed(3)})`;
        emergeCtx.fillRect(g.x * w, y, g.r, g.r);
      }
      requestAnimationFrame(step);
    }
    requestAnimationFrame(step);
  }

  async function termSetOpen(open) {
    const dock = $('term-dock');
    dock.classList.toggle('collapsed', !open);
    $('btn-term-toggle').textContent = open ? '▼ CLOSE' : '▲ OPEN';
    if (open) {
      if (!termCanvas.style.height || parseInt(termCanvas.style.height, 10) < 220) {
        termCanvas.style.height = '420px';
      }
      runEmergence();
      await termEnsureBoot();
      termCanvas.focus();
    }
  }
  $('btn-term-toggle').addEventListener('click', () =>
    termSetOpen($('term-dock').classList.contains('collapsed')));

  // 👻 HAUNTBOX: sealed-WASI gatekeeper (wasibox-server GREEN/RED cycle).
  // Not available in this build (wasibox infrastructure in F:\v3 only).
  $('btn-hauntbox').addEventListener('click', async () => {
    const wasClosed = $('term-dock').classList.contains('collapsed');
    if (wasClosed) await termSetOpen(true);
    const msg = "\rWrite-Host '`n[hauntbox] WASI sandbox demo not available in this build.`nRun F:\\v3 project for wasibox-server infrastructure.`n' -ForegroundColor Yellow\r";
    setTimeout(
      () => invokeCommand('term_write', { data: msg }),
      wasClosed ? 900 : 100,
    );
    termCanvas.focus();
  });

  // Accordion grip: drag the dock's top edge to size the glass; drag down past
  // the floor collapses it; double-click toggles. ResizeObserver re-deals the
  // ConPTY rows/cols on every height change.
  const termGrip = $('term-grip');
  let gripDrag = null;
  termGrip.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    termGrip.setPointerCapture(e.pointerId);
    const open = !$('term-dock').classList.contains('collapsed');
    gripDrag = { y: e.clientY, h: open ? termCanvas.clientHeight : 0 };
  });
  termGrip.addEventListener('pointermove', (e) => {
    if (!gripDrag) return;
    const h = Math.round(gripDrag.h + (gripDrag.y - e.clientY));
    if (h > 60) {
      if ($('term-dock').classList.contains('collapsed')) termSetOpen(true);
      const cap = Math.round(window.innerHeight * 0.7);
      termCanvas.style.height = Math.min(h, cap) + 'px';
    } else if (h < 30 && !$('term-dock').classList.contains('collapsed')) {
      termSetOpen(false);
    }
  });
  termGrip.addEventListener('pointerup', (e) => {
    gripDrag = null;
    try { termGrip.releasePointerCapture(e.pointerId); } catch (_) {}
  });
  termGrip.addEventListener('dblclick', () =>
    termSetOpen($('term-dock').classList.contains('collapsed')));

  // ── Sovereign telemetry ─────────────────────────────────────
  function updateStatus(status) {
    if (!status) return;
    const dp = $('daemon-pill'), sp = $('sidecar-pill');
    if (status.online) {
      dp.className = 'status-pill online'; dp.textContent = 'ONLINE';
      $('daemon-uptime').textContent = `uptime: ${status.uptime_secs}s`;
      $('context-health').textContent = `HEALTH: ${status.context_health}`;
      $('shi-score').textContent = `SHI: ${status.shi_score}`;
    } else {
      dp.className = 'status-pill offline'; dp.textContent = 'OFFLINE';
      $('daemon-uptime').textContent = '--';
      $('context-health').textContent = 'HEALTH: --';
      $('shi-score').textContent = 'SHI: --';
    }
    const up = status.sidecar_status === 'READY';
    sp.className = 'status-pill ' + (up ? 'online' : 'offline');
    sp.textContent = status.sidecar_status || 'OFFLINE';
    updateVram(status.vram);
  }

  // Driver-reported residency. No probe => the bar says so, never a zero fill
  // that would read as "the card is empty".
  function updateVram(v) {
    const fill = $('vram-fill'), text = $('vram-text');
    if (!fill || !text) return;
    if (!v) {
      fill.style.width = '0%';
      fill.className = 'absent';
      text.textContent = 'no probe';
      return;
    }
    const pct = Math.max(0, Math.min(100, v.used_pct | 0));
    fill.style.width = pct + '%';
    fill.className = pct >= 90 ? 'crit' : (pct >= 75 ? 'warn' : 'ok');
    text.textContent = `${v.used_mb} / ${v.total_mb} MB · ${pct}% · ${v.source}`;
    updatePred(v.predicted);
  }

  // The oracle's model of the 5-model fleet, drawn against the same card.
  // Amber when it fits on top of what is already resident, spark when it does not.
  function updatePred(p) {
    const bar = $('vram-pred'), txt = $('vram-pred-text');
    if (!bar || !txt) return;
    if (!p) { bar.style.width = '0%'; txt.textContent = '--'; return; }
    const pct = Math.max(0, Math.min(100, p.committed_pct | 0));
    bar.style.width = pct + '%';
    bar.className = p.fits ? 'pred' : 'pred-over';
    txt.textContent =
      `${p.committed_mb} MB fleet @ ${p.ctx_tokens} tok i8 ` +
      `(w ${p.weights_mb} + kv ${p.kv_mb} + oh ${p.overhead_mb}) · ` +
      `max ${p.max_ctx_tokens} tok · ${p.fits ? 'FITS' : 'OVER'}`;
  }

  // ── 5D HYPER-PROJECTION & SO(5) GIVENS DIRECT CONTROLS ──
  function tickSo5Spin() {
    if (so5State.autoSpin) {
      so5State.theta_zw = (so5State.theta_zw + 0.015) % (Math.PI * 2);
      so5State.phi_wv = (so5State.phi_wv + 0.010) % (Math.PI * 2);
      const sZw = $('slider-theta-zw');
      const sWv = $('slider-phi-wv');
      if (sZw) sZw.value = Math.round(so5State.theta_zw * 100);
      if (sWv) sWv.value = Math.round(so5State.phi_wv * 100);
      const vZw = $('val-theta-zw');
      const vWv = $('val-phi-wv');
      if (vZw) vZw.textContent = `${so5State.theta_zw.toFixed(2)} rad`;
      if (vWv) vWv.textContent = `${so5State.phi_wv.toFixed(2)} rad`;
      so5State.animId = requestAnimationFrame(tickSo5Spin);
    }
  }

  const sThetaZw = $('slider-theta-zw');
  if (sThetaZw) {
    sThetaZw.addEventListener('input', (e) => {
      so5State.theta_zw = parseFloat(e.target.value) / 100;
      const vZw = $('val-theta-zw');
      if (vZw) vZw.textContent = `${so5State.theta_zw.toFixed(2)} rad`;
      if (activeStarIdx >= 0 && !$('star-dossier').classList.contains('hidden')) {
        showDossier(activeStarIdx);
      }
    });
  }

  const sPhiWv = $('slider-phi-wv');
  if (sPhiWv) {
    sPhiWv.addEventListener('input', (e) => {
      so5State.phi_wv = parseFloat(e.target.value) / 100;
      const vWv = $('val-phi-wv');
      if (vWv) vWv.textContent = `${so5State.phi_wv.toFixed(2)} rad`;
      if (activeStarIdx >= 0 && !$('star-dossier').classList.contains('hidden')) {
        showDossier(activeStarIdx);
      }
    });
  }

  const sBeta = $('slider-beta-lorentz');
  if (sBeta) {
    sBeta.addEventListener('input', (e) => {
      so5State.beta_lorentz = parseFloat(e.target.value) / 100;
      const vBeta = $('val-beta-lorentz');
      if (vBeta) vBeta.textContent = `${so5State.beta_lorentz.toFixed(2)} c`;
      if (activeStarIdx >= 0 && !$('star-dossier').classList.contains('hidden')) {
        showDossier(activeStarIdx);
      }
    });
  }

  const btnFocusEarth = $('btn-focus-earth');
  if (btnFocusEarth) {
    btnFocusEarth.addEventListener('click', () => {
      cam5d.tx = 0;
      cam5d.ty = 0;
      cam5d.tz = 0;
      cam5d.distance = 220.0;
      cam5d.pitch = 0.35;
      cam5d.yaw = 0.0;
      scheduleRefreshSky();
    });
  }

  const btnSpinSo5 = $('btn-spin-so5');
  if (btnSpinSo5) {
    btnSpinSo5.addEventListener('click', () => {
      so5State.autoSpin = !so5State.autoSpin;
      btnSpinSo5.classList.toggle('active', so5State.autoSpin);
      if (so5State.autoSpin) {
        tickSo5Spin();
      } else if (so5State.animId) {
        cancelAnimationFrame(so5State.animId);
      }
    });
  }

  const btnReset5d = $('btn-reset-5d');
  if (btnReset5d) {
    btnReset5d.addEventListener('click', () => {
      so5State.theta_zw = 0;
      so5State.phi_wv = 0;
      so5State.beta_lorentz = 0;
      so5State.autoSpin = false;
      if (btnSpinSo5) btnSpinSo5.classList.remove('active');
      if (sThetaZw) sThetaZw.value = 0;
      if (sPhiWv) sPhiWv.value = 0;
      if (sBeta) sBeta.value = 0;
      const vZw = $('val-theta-zw');
      const vWv = $('val-phi-wv');
      const vBeta = $('val-beta-lorentz');
      if (vZw) vZw.textContent = '0.00 rad';
      if (vWv) vWv.textContent = '0.00 rad';
      if (vBeta) vBeta.textContent = '0.00 c';
      if (activeStarIdx >= 0 && !$('star-dossier').classList.contains('hidden')) {
        showDossier(activeStarIdx);
      }
    });
  }

  // ── HEADER NAVIGATION CONTROLS ──
  const btnNavSky = $('btn-nav-sky');
  const btnNavTerm = $('btn-nav-term');
  const btnTheory = $('btn-theory');

  function setActiveNavTab(activeBtn) {
    [btnNavSky, btnNavTerm, btnTheory].forEach((b) => {
      if (b) b.classList.toggle('active', b === activeBtn);
    });
  }

  if (btnNavSky) {
    btnNavSky.addEventListener('click', () => {
      setActiveNavTab(btnNavSky);
      if ($('theory-panel')) $('theory-panel').classList.add('hidden');
      termSetOpen(false);
    });
  }

  if (btnNavTerm) {
    btnNavTerm.addEventListener('click', () => {
      const isClosed = $('term-dock').classList.contains('collapsed');
      termSetOpen(isClosed);
      if (isClosed) {
        setActiveNavTab(btnNavTerm);
      } else {
        setActiveNavTab(btnNavSky);
      }
    });
  }

  // ── ♪ HARMONIC THEORY PANEL CONTROLS ──
  const theoryPanel = $('theory-panel');
  const theoryClose = $('theory-close');
  if (theoryPanel) makeDraggable(theoryPanel);

  if (btnTheory && theoryPanel) {
    btnTheory.addEventListener('click', () => {
      const isHidden = theoryPanel.classList.toggle('hidden');
      btnTheory.classList.toggle('active', !isHidden);
      if (!isHidden) {
        $('theory-key').textContent = 'CANONICAL A440';
        $('theory-scale').textContent = 'JUST INTONATION / 5D ASTROLABE';
        $('theory-blurb').textContent = 'Pythagorean & Just celestial scale harmonics mapped to 16 canonical sky anchors.';
        $('theory-notes').textContent = 'A, B, C#, D, E, F#, G# (Harmonic Lattice)';
        $('theory-beat').textContent = '120 BPM · 44.45M stars/s';
      }
    });
  }

  if (theoryClose && theoryPanel) {
    theoryClose.addEventListener('click', () => {
      theoryPanel.classList.add('hidden');
      if (btnTheory) btnTheory.classList.remove('active');
    });
  }

  const theoryAudition = $('theory-audition');
  if (theoryAudition) {
    theoryAudition.addEventListener('click', () => {
      if (typeof playHarmonicChord === 'function') {
        playHarmonicChord([440, 550, 660, 880]);
      }
    });
  }

  const theoryPlayBach = $('theory-play-bach');
  if (theoryPlayBach) {
    theoryPlayBach.addEventListener('click', () => {
      if (typeof playHarmonicChord === 'function') {
        playHarmonicChord([220, 277.18, 329.63, 440, 554.37]);
      }
    });
  }

  const theoryTuning = $('theory-tuning');
  if (theoryTuning) {
    let isA432 = false;
    theoryTuning.addEventListener('click', () => {
      isA432 = !isA432;
      theoryTuning.textContent = isA432 ? 'A432 (Verdi)' : 'A440 (ISO)';
      $('theory-ref').textContent = isA432 ? 'A432' : 'A440';
    });
  }

  const theoryVolSlider = $('theory-vol-slider');
  if (theoryVolSlider) {
    theoryVolSlider.addEventListener('input', (e) => {
      const val = e.target.value;
      const lbl = $('theory-vol-val');
      if (lbl) lbl.textContent = `${val}%`;
    });
  }

  async function setupTelemetry() {
    const t = getTauri();
    if (t && t.event && t.event.listen) {
      await t.event.listen('daemon-telemetry', (ev) => updateStatus(ev.payload));
    }
  }

  window.addEventListener('DOMContentLoaded', () => {
    sizeSpace();
    glSkyBoot();
    requestAnimationFrame(skyFrame);
    refreshSky();
    setupTelemetry();
    // Terminal starts closed (Sean 2026-08-26); first ▲ OPEN boots the ConPTY.
  });
})();
