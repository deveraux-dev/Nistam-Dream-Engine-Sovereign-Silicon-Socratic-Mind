#!/usr/bin/env python3
"""
Quantize a Hugging Face Gemma checkpoint into S13 balanced-ternary .s13m weight
files matching the on-disk format of s13_gemma_2b_m3/ and s13_gemma_9b_m3/.

Format verified against crates/gemma-s13/src/s13.rs (S13TensorView, S133 path)
and against the real header bytes of s13_gemma_2b_m3/blk_0_attn_k_weight.s13m:

    S133 header (20 bytes, all little-endian):
        magic        4 bytes   b"S133"
        out_features u32
        in_features  u32
        scale        f32       global per-tensor scale
        group_size   u32       input columns per scale group (64 on every seat)
    group_scales:    i16 LE * (out_features * ceil(in_features / group_size))
                      permyriad (1..=10000), row-major over the group grid
    packed_trits:    ceil(out_features * in_features / 5) bytes
                      5 trits/byte, base-243: byte = d0*81 + d1*27 + d2*9 + d3*3 + d4
                      where trit -1 -> digit 0, 0 -> digit 1, +1 -> digit 2
                      (row-major flatten over (row, col), matching
                      S13TensorView::get_trit's linear_idx = row*in_features+col)

Ternary quantization is BitNet-b1.58-style absmean rounding per 64-column group:
    alpha  = mean(|w|) over the group
    trit   = clip(round(w / alpha), -1, 1)
    weight ~= trit * (group_pmy/10000) * global_scale

Usage:
    python scripts/quantize_gemma_s13.py \
        --src G:\\path\\to\\gemma-3-27b-it \
        --dst s13_gemma_27b_m3 \
        --config 27b

Requires: numpy, safetensors  (pip install numpy safetensors)
"""

import argparse
import json
import struct
import sys
from pathlib import Path

import numpy as np
from safetensors import safe_open

GROUP_SIZE = 64

CONFIGS = {
    # d_model, n_heads, n_kv_heads, d_head, n_layers, d_ff
    "2b":  dict(d_model=2304, n_heads=8,  n_kv_heads=4,  d_head=256, n_layers=26, d_ff=9216),
    "9b":  dict(d_model=3584, n_heads=16, n_kv_heads=8,  d_head=256, n_layers=42, d_ff=14336),
    "27b": dict(d_model=4608, n_heads=32, n_kv_heads=16, d_head=128, n_layers=46, d_ff=14336),
}

# (hf suffix under model.layers.{i}., blk kind name, out_dim key, in_dim key)
TENSOR_SPECS = [
    ("self_attn.q_proj.weight", "attn_q",      "q_out",  "d_model"),
    ("self_attn.k_proj.weight", "attn_k",      "kv_out", "d_model"),
    ("self_attn.v_proj.weight", "attn_v",      "kv_out", "d_model"),
    ("self_attn.o_proj.weight", "attn_output", "d_model", "q_out"),
    ("mlp.gate_proj.weight",    "ffn_gate",    "d_ff",   "d_model"),
    ("mlp.up_proj.weight",      "ffn_up",      "d_ff",   "d_model"),
    ("mlp.down_proj.weight",    "ffn_down",    "d_model", "d_ff"),
]


class ShardedCheckpoint:
    """Zero-copy-ish reader over a (possibly sharded) safetensors checkpoint dir."""

    def __init__(self, src: Path):
        index_path = src / "model.safetensors.index.json"
        if index_path.exists():
            index = json.loads(index_path.read_text())
            self.weight_map = index["weight_map"]
        else:
            single = src / "model.safetensors"
            if not single.exists():
                raise FileNotFoundError(f"No model.safetensors(.index.json) under {src}")
            # Enumerate tensor names from the single file itself.
            with safe_open(str(single), framework="numpy") as f:
                self.weight_map = {k: "model.safetensors" for k in f.keys()}
        self.src = src
        self._handles = {}

    def _handle(self, filename: str):
        if filename not in self._handles:
            self._handles[filename] = safe_open(str(self.src / filename), framework="numpy")
        return self._handles[filename]

    def get(self, name: str) -> np.ndarray:
        filename = self.weight_map.get(name)
        if filename is None:
            raise KeyError(f"Tensor not found in checkpoint: {name}")
        return self._handle(filename).get_tensor(name)


def quantize_matrix(w: np.ndarray) -> tuple[np.ndarray, np.ndarray, float]:
    """Quantize a (out_features, in_features) f32/bf16 matrix to (trits, group_pmy, global_scale)."""
    w = w.astype(np.float32)
    out_f, in_f = w.shape
    n_groups = (in_f + GROUP_SIZE - 1) // GROUP_SIZE

    group_alpha = np.zeros((out_f, n_groups), dtype=np.float32)
    trits = np.zeros((out_f, in_f), dtype=np.int8)

    for g in range(n_groups):
        c0, c1 = g * GROUP_SIZE, min((g + 1) * GROUP_SIZE, in_f)
        block = w[:, c0:c1]
        alpha = np.mean(np.abs(block), axis=1)
        alpha = np.where(alpha < 1e-12, 1e-12, alpha)
        group_alpha[:, g] = alpha
        scaled = block / alpha[:, None]
        trits[:, c0:c1] = np.clip(np.round(scaled), -1, 1).astype(np.int8)

    global_scale = float(np.max(group_alpha))
    if global_scale <= 0.0:
        global_scale = 1.0
    group_pmy = np.clip(np.round(group_alpha / global_scale * 10000.0), 1, 10000).astype(np.int16)

    return trits, group_pmy, global_scale


def pack_trits(trits_flat: np.ndarray) -> bytes:
    """Pack a flat int8 trit array (-1,0,1) into base-243 bytes, 5 trits/byte."""
    n = trits_flat.shape[0]
    pad = (-n) % 5
    if pad:
        trits_flat = np.concatenate([trits_flat, np.zeros(pad, dtype=np.int8)])  # padding trit = 0
    digits = (trits_flat + 1).astype(np.uint32)  # -1,0,1 -> 0,1,2
    digits = digits.reshape(-1, 5)
    bytes_arr = (
        digits[:, 0] * 81 + digits[:, 1] * 27 + digits[:, 2] * 9 + digits[:, 3] * 3 + digits[:, 4]
    ).astype(np.uint8)
    return bytes_arr.tobytes()


def write_s13m(path: Path, out_f: int, in_f: int, global_scale: float,
                group_pmy: np.ndarray, trits: np.ndarray) -> None:
    header = b"S133"
    header += struct.pack("<I", out_f)
    header += struct.pack("<I", in_f)
    header += struct.pack("<f", global_scale)
    header += struct.pack("<I", GROUP_SIZE)
    assert len(header) == 20

    scales_bytes = group_pmy.astype("<i2").tobytes()  # row-major (out_f, n_groups)
    trits_bytes = pack_trits(trits.reshape(-1))

    path.write_bytes(header + scales_bytes + trits_bytes)


def dims(cfg: dict) -> dict:
    d = dict(cfg)
    d["q_out"] = cfg["n_heads"] * cfg["d_head"]
    d["kv_out"] = cfg["n_kv_heads"] * cfg["d_head"]
    return d


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--src", required=True, type=Path, help="HF checkpoint dir (safetensors)")
    ap.add_argument("--dst", required=True, type=Path, help="Output dir, e.g. s13_gemma_27b_m3")
    ap.add_argument("--config", required=True, choices=CONFIGS.keys())
    ap.add_argument("--layers", type=int, default=None, help="Override n_layers (default from --config)")
    args = ap.parse_args()

    cfg = dims(CONFIGS[args.config])
    n_layers = args.layers or cfg["n_layers"]

    args.dst.mkdir(parents=True, exist_ok=True)
    ckpt = ShardedCheckpoint(args.src)

    print(f"[quantize_gemma_s13] config={args.config} n_layers={n_layers} d_model={cfg['d_model']}")
    print(f"[quantize_gemma_s13] src={args.src}")
    print(f"[quantize_gemma_s13] dst={args.dst}")

    for i in range(n_layers):
        for hf_suffix, kind, out_key, in_key in TENSOR_SPECS:
            hf_name = f"model.layers.{i}.{hf_suffix}"
            out_f, in_f = cfg[out_key], cfg[in_key]

            w = ckpt.get(hf_name)
            if w.shape != (out_f, in_f):
                print(f"[quantize_gemma_s13] WARNING: {hf_name} shape {w.shape} "
                      f"!= expected ({out_f}, {in_f}); using actual shape", file=sys.stderr)
                out_f, in_f = w.shape

            trits, group_pmy, global_scale = quantize_matrix(w)
            out_path = args.dst / f"blk_{i}_{kind}_weight.s13m"
            write_s13m(out_path, out_f, in_f, global_scale, group_pmy, trits)

        print(f"[quantize_gemma_s13]   layer {i+1}/{n_layers} done")

    print("[quantize_gemma_s13] SUCCESS")
    print(f"  git lfs track '*.s13m'   # already tracked via .gitattributes")
    print(f"  git add {args.dst} && git commit -m 'Add: S13 quantized Gemma 27B weights'")
    return 0


if __name__ == "__main__":
    sys.exit(main())
