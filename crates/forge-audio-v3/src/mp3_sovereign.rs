//! mp3_sovereign — a from-scratch MPEG-1 Layer III decoder (the symphonia replacement),
//! built in stages and differential-validated against a real mp3 library.
//!
//! The decode pipeline, in order (each a later stage on this foundation):
//!   1. **[this file today]** compressed-side reader: MSB-first [`BitReader`], frame
//!      [`Mp3Header`], and the Layer III [`SideInfo`] — the exact bitstream structure.
//!   2. main-data reservoir assembly (`main_data_begin` back-pointer) + Huffman decode.
//!   3. requantize (global_gain / scalefactors) + stereo (MS/intensity).
//!   4. reorder (short blocks) + alias reduction.
//!   5. IMDCT (long/short windows) + overlap-add.
//!   6. polyphase synthesis filterbank → PCM.
//!
//! Validation: `dsp::load_audio` (symphonia) is the reference oracle. The env-gated
//! `differential_*` test decodes a real file from the caller's library both ways and
//! compares — reverse-engineering the format against ground truth while building.

/// MSB-first bit reader over a byte slice. The mp3 bitstream is read big-end, bit-first;
/// reads past the end yield zero bits (the caller bounds-checks frame length — this
/// never panics, honest-empty rather than crash).
pub struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, bit_pos: 0 }
    }
    pub fn bit_pos(&self) -> usize {
        self.bit_pos
    }
    /// Read `n` bits (0..=32) MSB-first.
    pub fn read(&mut self, n: u32) -> u32 {
        let mut v = 0u32;
        for _ in 0..n {
            let byte = self.bit_pos / 8;
            let bit = 7 - (self.bit_pos % 8);
            let b = if byte < self.data.len() { (self.data[byte] >> bit) & 1 } else { 0 };
            v = (v << 1) | b as u32;
            self.bit_pos += 1;
        }
        v
    }
    pub fn skip(&mut self, n: usize) {
        self.bit_pos += n;
    }
}

const BITRATE_KBPS_MPEG1_L3: [u32; 16] =
    [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0];
const SAMPLE_RATE_MPEG1: [u32; 4] = [44100, 48000, 32000, 0];

/// The four MPEG channel modes (bits 7-6 of header byte 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelMode {
    Stereo,
    JointStereo,
    DualChannel,
    Mono,
}
impl ChannelMode {
    pub fn channels(self) -> usize {
        if self == ChannelMode::Mono {
            1
        } else {
            2
        }
    }
}

/// A parsed MPEG-1 Layer III frame header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mp3Header {
    pub bitrate_kbps: u32,
    pub sample_rate: u32,
    pub padding: u32,
    pub mode: ChannelMode,
    /// True when the frame carries a 16-bit CRC after the header (protection bit = 0).
    pub crc: bool,
}

impl Mp3Header {
    /// Parse a 4-byte header. `None` if it is not a valid MPEG-1 Layer III sync+header
    /// (a loud reject, never a silent misparse).
    pub fn parse(h: &[u8]) -> Option<Self> {
        if h.len() < 4 || h[0] != 0xFF || (h[1] & 0xE0) != 0xE0 {
            return None;
        }
        let version = (h[1] >> 3) & 0x3; // 3 = MPEG1
        let layer = (h[1] >> 1) & 0x3; // 1 = Layer III
        if version != 3 || layer != 1 {
            return None;
        }
        let crc = (h[1] & 0x1) == 0; // protection bit 0 → CRC present
        let bitrate_kbps = BITRATE_KBPS_MPEG1_L3[((h[2] >> 4) & 0xF) as usize];
        let sample_rate = SAMPLE_RATE_MPEG1[((h[2] >> 2) & 0x3) as usize];
        if bitrate_kbps == 0 || sample_rate == 0 {
            return None;
        }
        let padding = ((h[2] >> 1) & 0x1) as u32;
        let mode = match (h[3] >> 6) & 0x3 {
            0 => ChannelMode::Stereo,
            1 => ChannelMode::JointStereo,
            2 => ChannelMode::DualChannel,
            _ => ChannelMode::Mono,
        };
        Some(Self { bitrate_kbps, sample_rate, padding, mode, crc })
    }

    /// Total frame length in bytes (header included). MPEG-1 Layer III:
    /// `144 * bitrate / sample_rate + padding`, bitrate in bits/s.
    pub fn frame_size(&self) -> usize {
        (144 * self.bitrate_kbps * 1000 / self.sample_rate + self.padding) as usize
    }

    /// Side-information length: MPEG-1 is 17 bytes mono, 32 bytes otherwise.
    pub fn side_info_bytes(&self) -> usize {
        if self.mode == ChannelMode::Mono {
            17
        } else {
            32
        }
    }

    /// PCM samples this frame yields per channel (constant for Layer III).
    pub const SAMPLES_PER_FRAME: usize = 1152;
}

/// Per-granule, per-channel Layer III side information.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GranuleChannel {
    pub part2_3_length: u32,
    pub big_values: u32,
    pub global_gain: u32,
    pub scalefac_compress: u32,
    pub window_switching: bool,
    pub block_type: u32,
    pub mixed_block: bool,
    pub table_select: [u32; 3],
    pub subblock_gain: [u32; 3],
    pub region0_count: u32,
    pub region1_count: u32,
    pub preflag: u32,
    pub scalefac_scale: u32,
    pub count1table_select: u32,
}

/// A frame's full Layer III side information (MPEG-1: 2 granules).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideInfo {
    pub main_data_begin: u32,
    pub scfsi: [[u32; 4]; 2],
    pub granules: [[GranuleChannel; 2]; 2],
    pub channels: usize,
}

/// Parse the MPEG-1 Layer III side information block (17/32 bytes).
pub fn parse_side_info(bytes: &[u8], mode: ChannelMode) -> SideInfo {
    let channels = mode.channels();
    let mut r = BitReader::new(bytes);
    let main_data_begin = r.read(9);
    r.skip(if channels == 1 { 5 } else { 3 }); // private bits
    let mut scfsi = [[0u32; 4]; 2];
    for scfsi_ch in scfsi.iter_mut().take(channels) {
        for band in scfsi_ch.iter_mut() {
            *band = r.read(1);
        }
    }
    let mut granules = [[GranuleChannel::default(); 2]; 2];
    for gr in 0..2 {
        for ch in 0..channels {
            let g = &mut granules[gr][ch];
            g.part2_3_length = r.read(12);
            g.big_values = r.read(9);
            g.global_gain = r.read(8);
            g.scalefac_compress = r.read(4);
            g.window_switching = r.read(1) == 1;
            if g.window_switching {
                g.block_type = r.read(2);
                g.mixed_block = r.read(1) == 1;
                for t in g.table_select.iter_mut().take(2) {
                    *t = r.read(5);
                }
                for sg in g.subblock_gain.iter_mut() {
                    *sg = r.read(3);
                }
                // Region counts are implied for short/mixed blocks.
                g.region0_count = if g.block_type == 2 && !g.mixed_block { 8 } else { 7 };
                g.region1_count = 20 - g.region0_count;
            } else {
                for t in g.table_select.iter_mut() {
                    *t = r.read(5);
                }
                g.region0_count = r.read(4);
                g.region1_count = r.read(3);
            }
            g.preflag = r.read(1);
            g.scalefac_scale = r.read(1);
            g.count1table_select = r.read(1);
        }
    }
    SideInfo { main_data_begin, scfsi, granules, channels }
}

/// A frame's header + side info + a pointer to its main data (Huffman-coded payload).
#[derive(Clone, Debug)]
pub struct Mp3Frame {
    pub header: Mp3Header,
    pub side_info: SideInfo,
    /// Offset into the input where this frame's main data begins.
    pub main_data_offset: usize,
    pub main_data_len: usize,
}

/// STAGE 1: parse every MPEG-1 Layer III frame's header + side information. No PCM yet —
/// this is the compressed structure the decode stages consume. Skips an ID3v2 tag and
/// resyncs past junk between frames.
pub fn parse_frames(bytes: &[u8]) -> Vec<Mp3Frame> {
    let mut frames = Vec::new();
    let mut i = 0usize;
    if bytes.len() >= 10 && &bytes[0..3] == b"ID3" {
        let sz = ((bytes[6] as usize & 0x7F) << 21)
            | ((bytes[7] as usize & 0x7F) << 14)
            | ((bytes[8] as usize & 0x7F) << 7)
            | (bytes[9] as usize & 0x7F);
        i = 10 + sz;
    }
    while i + 4 <= bytes.len() {
        let Some(header) = Mp3Header::parse(&bytes[i..]) else {
            i += 1;
            continue;
        };
        let fsize = header.frame_size();
        if fsize < 4 || i + fsize > bytes.len() {
            break;
        }
        let crc_bytes = if header.crc { 2 } else { 0 };
        let si_start = i + 4 + crc_bytes;
        let si_bytes = header.side_info_bytes();
        if si_start + si_bytes > bytes.len() {
            break;
        }
        let side_info = parse_side_info(&bytes[si_start..si_start + si_bytes], header.mode);
        let main_data_offset = si_start + si_bytes;
        let main_data_len = (i + fsize).saturating_sub(main_data_offset);
        frames.push(Mp3Frame { header, side_info, main_data_offset, main_data_len });
        i += fsize;
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitreader_reads_msb_first() {
        let data = [0xB2u8, 0x4Du8]; // 1011_0010 0100_1101
        let mut r = BitReader::new(&data);
        assert_eq!(r.read(4), 0b1011);
        assert_eq!(r.read(4), 0b0010);
        assert_eq!(r.read(8), 0b0100_1101);
        assert_eq!(r.bit_pos(), 16);
        assert_eq!(r.read(4), 0, "reads past the end are zero, not a panic");
    }

    #[test]
    fn header_parses_a_valid_mpeg1_layer3_frame() {
        // 0xFF 0xFB = sync + MPEG1 + Layer III + no CRC. 0x90 = 128kbps, 44100, no pad.
        // 0x00 = stereo.
        let h = Mp3Header::parse(&[0xFF, 0xFB, 0x90, 0x00]).expect("valid header");
        assert_eq!(h.bitrate_kbps, 128);
        assert_eq!(h.sample_rate, 44100);
        assert_eq!(h.mode, ChannelMode::Stereo);
        assert_eq!(h.padding, 0);
        assert_eq!(h.frame_size(), 417);
        assert_eq!(h.side_info_bytes(), 32);
        assert!(!h.crc, "0xFB protection bit = 1 → no CRC");
        // Rejects: wrong sync, and MPEG2/Layer II.
        assert!(Mp3Header::parse(&[0x00, 0xFB, 0x90, 0x00]).is_none());
        assert!(Mp3Header::parse(&[0xFF, 0xF3, 0x90, 0x00]).is_none()); // layer II
    }

    #[test]
    fn mono_frame_has_17_byte_side_info() {
        // 0xC0 = mono; 0xFA has protection bit = 0 → CRC present.
        let h = Mp3Header::parse(&[0xFF, 0xFA, 0x90, 0xC0]).expect("valid mono header");
        assert_eq!(h.mode, ChannelMode::Mono);
        assert_eq!(h.side_info_bytes(), 17);
        assert!(h.crc, "0xFA protection bit = 0 → CRC present");
    }

    #[test]
    fn side_info_reads_main_data_begin() {
        // First 9 bits = 1_0000_0000 = 256.
        let mut si = vec![0x80u8, 0x00];
        si.resize(32, 0);
        let s = parse_side_info(&si, ChannelMode::Stereo);
        assert_eq!(s.main_data_begin, 256);
        assert_eq!(s.channels, 2);
    }

    #[test]
    fn parse_frames_finds_one_synthetic_frame() {
        let mut m = vec![0xFFu8, 0xFB, 0x90, 0x00]; // stereo, no CRC, 417-byte frame
        m.resize(417, 0);
        let frames = parse_frames(&m);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].header.bitrate_kbps, 128);
        // main data starts after header(4) + side info(32) = offset 36 (no CRC).
        assert_eq!(frames[0].main_data_offset, 36);
        assert_eq!(frames[0].main_data_len, 417 - 36);
    }

    /// Differential harness — the reverse-engineering loop. Set `MP3_DIFF_PATH` to an
    /// mp3 in your library; this parses it sovereignly and cross-checks the frame count
    /// against symphonia's decoded length (1152 samples/frame). Skips when unset so it
    /// never blocks CI, but is a real oracle when pointed at a file.
    #[test]
    fn differential_frame_count_vs_symphonia() {
        let Ok(path) = std::env::var("MP3_DIFF_PATH") else {
            eprintln!("[mp3-diff] MP3_DIFF_PATH unset — skipping (point it at an mp3 to validate)");
            return;
        };
        let bytes = std::fs::read(&path).expect("read the mp3");
        let ours = parse_frames(&bytes);
        assert!(!ours.is_empty(), "sovereign parser found no frames in {path}");
        let reference = crate::dsp::load_audio(&path).expect("symphonia decodes the reference");
        let expected = reference.len() / Mp3Header::SAMPLES_PER_FRAME;
        let got = ours.len();
        eprintln!("[mp3-diff] {path}: sovereign frames={got}, symphonia ~frames={expected}");
        assert!(
            (got as i64 - expected as i64).abs() <= 2,
            "frame-count drift: sovereign {got} vs symphonia {expected}",
        );
    }
}
