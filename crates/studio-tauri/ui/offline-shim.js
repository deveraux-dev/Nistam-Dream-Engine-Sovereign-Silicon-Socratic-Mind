// Offline __TAURI__ shim for the single-file giveaway build.
// Serves the star lane from inlined gzip+base64 blobs; every other command
// returns null, which app.js already null-guards.
(function () {
  'use strict';

  const G = window.__FORGE_GIVEAWAY__;
  if (!G) return;

  const HDR = 16;
  const LUT = 256 * 4;
  const REC = 17;
  const INK_MAG_BANDS = 16;
  const EDMONTON_LON_DEG = -113.49;
  const TARGET_MAX = 150.0;
  const NEAR_CLIP = 0.1;
  const FAR_CLIP = 1500.0;

  function b64ToBytes(s) {
    const bin = atob(s);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }

  async function gunzip(b64) {
    const packed = b64ToBytes(b64);
    if (typeof DecompressionStream !== 'function') return packed.buffer;
    const ds = new DecompressionStream('gzip');
    const w = ds.writable.getWriter();
    w.write(packed);
    w.close();
    const parts = [];
    let total = 0;
    const r = ds.readable.getReader();
    for (;;) {
      const { done, value } = await r.read();
      if (done) break;
      parts.push(value);
      total += value.length;
    }
    const out = new Uint8Array(total);
    let off = 0;
    for (const p of parts) { out.set(p, off); off += p.length; }
    return out.buffer;
  }

  function starRadius(distPc) {
    if (distPc === 0) return 400.0;
    return Math.min(400.0, 55.0 + (345.0 * Math.log(1.0 + distPc)) / Math.log(2001.0));
  }

  function starVis(mag) {
    const flux = Math.pow(10, -0.4 * (mag + 1.46));
    return Math.min(1.0, Math.max(0.05, Math.pow(flux, 0.28)));
  }

  function magNorm(pmy) {
    const a = Math.min(10500, Math.max(0, 4000 - Math.trunc(pmy / 10)));
    return Math.trunc((a * 1000) / 10500);
  }

  function magBand(pmy) {
    return Math.min(INK_MAG_BANDS - 1, Math.trunc((magNorm(pmy) * (INK_MAG_BANDS - 1)) / 1000));
  }

  function mod360(x) { return ((x % 360) + 360) % 360; }

  function julianDateNow() { return Date.now() / 86400000 + 2440587.5; }

  function lstDegrees(jd, lonDeg) {
    const d = jd - 2451545.0;
    return mod360(mod360(280.46061837 + 360.98564736629 * d) + lonDeg);
  }

  const S = {};

  function hygCounts(dv) {
    return { stars: dv.getUint32(8, true), anomalies: dv.getUint32(12, true) };
  }

  function readStar(i) {
    const o = HDR + LUT + i * REC;
    return {
      raU32: S.dv.getUint32(o, true),
      decI32: S.dv.getInt32(o + 4, true),
      magPmy: S.dv.getInt32(o + 8, true),
      dist: S.dv.getUint16(o + 12, true),
      teff: S.bytes[o + 14],
      lore: S.bytes[o + 16],
    };
  }

  // Port of main.rs::bake_sky_vbo — same layout viewSkyVbo() reads.
  function bakeSkyVbo() {
    const n = S.starCount + 1;
    const out = new ArrayBuffer(4 + n * 48);
    const dv = new DataView(out);
    dv.setUint32(0, n, true);
    let vo = 4;
    const raO = 4 + n * 32;
    const decO = raO + n * 4;
    const magO = decO + n * 4;
    const distO = magO + n * 4;
    const teffO = distO + n * 2;
    const loreO = teffO + n;
    for (let i = 0; i < S.starCount; i++) {
      const s = readStar(i);
      const ra = (s.raU32 / 4294967295) * Math.PI * 2;
      const dec = (s.decI32 / 2147483647) * (Math.PI / 2);
      const mag = s.magPmy / 10000;
      const r = starRadius(s.dist);
      const ink = (s.teff * INK_MAG_BANDS + magBand(s.magPmy)) * 3;
      dv.setFloat32(vo, r * Math.cos(dec) * Math.cos(ra), true); vo += 4;
      dv.setFloat32(vo, r * Math.sin(dec), true); vo += 4;
      dv.setFloat32(vo, r * Math.cos(dec) * Math.sin(ra), true); vo += 4;
      dv.setFloat32(vo, S.ink[ink] / 255, true); vo += 4;
      dv.setFloat32(vo, S.ink[ink + 1] / 255, true); vo += 4;
      dv.setFloat32(vo, S.ink[ink + 2] / 255, true); vo += 4;
      dv.setFloat32(vo, starVis(mag), true); vo += 4;
      dv.setFloat32(vo, (i * 0.37) % 2.0, true); vo += 4;
      dv.setFloat32(raO + i * 4, ra, true);
      dv.setFloat32(decO + i * 4, dec, true);
      dv.setFloat32(magO + i * 4, mag, true);
      dv.setUint16(distO + i * 2, s.dist, true);
      dv.setUint8(teffO + i, s.teff);
      dv.setUint8(loreO + i, s.lore);
    }
    for (const v of [0.0, 0.0, 0.0, 1.0, 0.96, 0.88, 1.0, -1.0]) { dv.setFloat32(vo, v, true); vo += 4; }
    const sol = S.starCount;
    dv.setFloat32(raO + sol * 4, 0, true);
    dv.setFloat32(decO + sol * 4, 0, true);
    dv.setFloat32(magO + sol * 4, -26.7, true);
    dv.setUint16(distO + sol * 2, 0, true);
    dv.setUint8(teffO + sol, 25);
    dv.setUint8(loreO + sol, 255);
    return out;
  }

  // Port of camera5d.rs::view_proj — column-major, flattened for app.js.
  function viewProj(distance, pitch, yaw, roll, fovDeg, target, aspect) {
    const dir = [Math.cos(pitch) * Math.sin(yaw), Math.sin(pitch), Math.cos(pitch) * Math.cos(yaw)];
    const eye = [target[0] + dir[0] * distance, target[1] + dir[1] * distance, target[2] + dir[2] * distance];
    const fwd = norm3([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
    const up = rodrigues([0, 1, 0], fwd, roll);
    const right = norm3(cross3(fwd, up));
    const tu = cross3(right, fwd);
    const view = [
      [right[0], tu[0], -fwd[0], 0],
      [right[1], tu[1], -fwd[1], 0],
      [right[2], tu[2], -fwd[2], 0],
      [-dot3(right, eye), -dot3(tu, eye), dot3(fwd, eye), 1],
    ];
    const fovRad = (Math.min(179, Math.max(1, fovDeg)) * Math.PI) / 180;
    const f = 1.0 / Math.tan(fovRad * 0.5);
    const a = Math.max(1e-4, aspect);
    const proj = [
      [f / a, 0, 0, 0],
      [0, f, 0, 0],
      [0, 0, FAR_CLIP / (NEAR_CLIP - FAR_CLIP), -1],
      [0, 0, (NEAR_CLIP * FAR_CLIP) / (NEAR_CLIP - FAR_CLIP), 0],
    ];
    const out = [[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]];
    for (let col = 0; col < 4; col++) {
      for (let row = 0; row < 4; row++) {
        let s = 0;
        for (let k = 0; k < 4; k++) s += proj[k][row] * view[col][k];
        out[col][row] = s;
      }
    }
    return { vp: out, eye };
  }

  function dot3(a, b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; }
  function cross3(a, b) {
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  }
  function norm3(v) {
    const l = Math.sqrt(dot3(v, v));
    return l > 1e-6 ? [v[0] / l, v[1] / l, v[2] / l] : [0, 0, 1];
  }
  function rodrigues(v, axis, angle) {
    const s = Math.sin(angle), c = Math.cos(angle);
    const kv = dot3(axis, v), kxv = cross3(axis, v);
    return [
      v[0] * c + kxv[0] * s + axis[0] * kv * (1 - c),
      v[1] * c + kxv[1] * s + axis[1] * kv * (1 - c),
      v[2] * c + kxv[2] * s + axis[2] * kv * (1 - c),
    ];
  }

  function starmap5d(a) {
    const clamp = (v) => Math.min(TARGET_MAX, Math.max(-TARGET_MAX, v || 0));
    const target = [clamp(a.tx), clamp(a.ty), clamp(a.tz)];
    const { vp } = viewProj(a.distance, a.pitch, a.yaw, a.roll, a.fovDeg, target, Math.max(0.1, a.aspect));
    const lstDeg = lstDegrees(julianDateNow(), EDMONTON_LON_DEG);
    const lstRad = (lstDeg * Math.PI) / 180;
    const lc = Math.cos(lstRad), ls = Math.sin(lstRad);
    const stars = [];
    for (let k = 0; k < S.subsetCount; k++) {
      const full = S.subDv.getUint32(k * 12, true);
      const colorRgba = S.subDv.getUint32(k * 12 + 4, true);
      const milliHz = S.subDv.getUint32(k * 12 + 8, true);
      const s = readStar(full);
      const ra = (s.raU32 / 4294967295) * Math.PI * 2;
      const dec = (s.decI32 / 2147483647) * (Math.PI / 2);
      const r = starRadius(s.dist);
      const p0 = [r * Math.cos(dec) * Math.cos(ra), r * Math.sin(dec), r * Math.cos(dec) * Math.sin(ra)];
      const p = [p0[0] * lc + p0[2] * ls, p0[1], -p0[0] * ls + p0[2] * lc];
      const cl = [0, 0, 0, 0];
      for (let row = 0; row < 4; row++) {
        cl[row] = vp[0][row] * p[0] + vp[1][row] * p[1] + vp[2][row] * p[2] + vp[3][row];
      }
      const w = cl[3];
      const ok = Math.abs(w) > 1e-6;
      const sx = ok ? cl[0] / w : 0;
      const sy = ok ? cl[1] / w : 0;
      const depth = ok ? cl[2] / w : -1;
      stars.push({
        idx: k, name: '', milli_hz: milliHz, color_rgba: colorRgba, mag_pmy: s.magPmy,
        sx, sy, depth,
        visible: w > 0 && depth >= 0 && depth <= 1 && Math.abs(sx) <= 1.2 && Math.abs(sy) <= 1.2,
        wx: p[0], wy: p[1], wz: p[2],
      });
    }
    return {
      stars,
      camera: { distance: a.distance, pitch: a.pitch, yaw: a.yaw, roll: a.roll, fov_deg: a.fovDeg },
      lst_deg: lstDeg,
      view_proj: vp,
      target,
    };
  }

  // Port of main.rs::designation_at — HYGN arena + one u32 offset per star.
  function designationAt(idx) {
    if (idx < 0 || idx >= S.starCount || S.hygnAt < 0) return '';
    const o = S.hygnAt + 8 + S.arenaLen + idx * 4;
    if (o + 4 > S.bytes.length) return '';
    const off = S.hygnAt + 8 + S.dv.getUint32(o, true);
    let end = off;
    while (end < S.hygnAt + 8 + S.arenaLen && S.bytes[end] !== 0) end++;
    return S.dec.decode(S.bytes.subarray(off, end));
  }

  async function init() {
    const buf = await gunzip(G.hyg);
    S.bytes = new Uint8Array(buf);
    S.dv = new DataView(buf);
    S.dec = new TextDecoder();
    const c = hygCounts(S.dv);
    S.starCount = c.stars;
    const sec = HDR + LUT + c.stars * REC + c.anomalies * 12;
    const tag = sec + 4 <= S.bytes.length
      ? String.fromCharCode(S.bytes[sec], S.bytes[sec + 1], S.bytes[sec + 2], S.bytes[sec + 3])
      : '';
    S.hygnAt = tag === 'HYGN' ? sec : -1;
    S.arenaLen = S.hygnAt >= 0 ? S.dv.getUint32(sec + 4, true) : 0;

    S.ink = new Uint8Array(await gunzip(G.ink));
    const sub = await gunzip(G.subset);
    S.subDv = new DataView(sub);
    S.subsetCount = sub.byteLength / 12;
    S.notes = new Uint8Array(await gunzip(G.notes));
    S.refAHz = (G.refAMhz || 440_000) / 1000;
    S.vbo = bakeSkyVbo();
  }

  const ready = init();

  function handle(cmd, args) {
    switch (cmd) {
      case 'get_sky_vbo': return S.vbo;
      case 'get_starmap_5d': return starmap5d(args);
      case 'get_sky_chart': return G.chart;
      case 'star_designation': return designationAt(args.idx | 0);
      // The bake carries the MIDI note, not a frequency — hardware is a
      // discrete 12-TET engine. The crossing into millihertz happens HERE, once,
      // at the audio edge, where a float is the right tool. Never as an integer
      // retune on the frequency itself.
      case 'star_voice': {
        const i = args.idx | 0;
        if (i < 0 || i >= S.starCount) return 0;
        const note = S.notes[i];
        return Math.round(S.refAHz * Math.pow(2, (note - 69) / 12) * 1000);
      }
      case 'star_note': {
        const i = args.idx | 0;
        return i >= 0 && i < S.starCount ? S.notes[i] : 0;
      }
      default: return null;
    }
  }

  window.__TAURI__ = {
    core: {
      invoke: async (cmd, args) => {
        await ready;
        try { return handle(cmd, args || {}); } catch (e) {
          console.warn(`[giveaway] ${cmd} failed:`, e);
          return null;
        }
      },
    },
    event: { listen: async () => () => {} },
  };
})();
