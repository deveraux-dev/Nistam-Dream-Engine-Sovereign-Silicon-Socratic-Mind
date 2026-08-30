//! Compress — LEB128 varint codec + a compact title pack. A tiny archival blob
//! distinct from JSON; round-trips a book's chapter titles.

use crate::book::Book;

/// Append `v` as an unsigned LEB128 varint.
pub fn write_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

/// Read an unsigned LEB128 varint from `data` at `*pos`, advancing it.
pub fn read_varint(data: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = *data.get(*pos)?;
        *pos += 1;
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
}

/// Pack a book's chapter titles into a varint-framed blob.
pub fn pack_titles(book: &Book) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, book.spine.chapters.len() as u64);
    for ch in &book.spine.chapters {
        let t = ch.title().as_bytes();
        write_varint(&mut out, t.len() as u64);
        out.extend_from_slice(t);
    }
    out
}

/// Unpack the titles written by [`pack_titles`].
pub fn unpack_titles(data: &[u8]) -> Vec<String> {
    let mut pos = 0;
    let mut titles = Vec::new();
    let Some(n) = read_varint(data, &mut pos) else {
        return titles;
    };
    for _ in 0..n {
        let Some(len) = read_varint(data, &mut pos) else {
            break;
        };
        let len = len as usize;
        if pos + len > data.len() {
            break;
        }
        if let Ok(s) = std::str::from_utf8(&data[pos..pos + len]) {
            titles.push(s.to_string());
        }
        pos += len;
    }
    titles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;

    #[test]
    fn varint_round_trips() {
        for v in [0u64, 1, 127, 128, 300, 16_384, u32::MAX as u64, u64::MAX] {
            let mut buf = Vec::new();
            write_varint(&mut buf, v);
            let mut pos = 0;
            assert_eq!(read_varint(&buf, &mut pos), Some(v));
            assert_eq!(pos, buf.len());
        }
    }

    #[test]
    fn titles_pack_and_unpack() {
        let mut b = Book::new("A", "d");
        b.open_chapter(AtlasSection::Items, "The Belt");
        b.open_chapter(AtlasSection::Weather, "Skies of Æther");
        let blob = pack_titles(&b);
        assert_eq!(unpack_titles(&blob), vec!["The Belt".to_string(), "Skies of Æther".to_string()]);
    }

    #[test]
    fn truncated_blob_is_safe() {
        assert!(unpack_titles(&[]).is_empty());
        assert!(unpack_titles(&[5, 200]).len() <= 5); // claims 5, no data — no panic
    }
}
