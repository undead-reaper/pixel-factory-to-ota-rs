// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use std::io::{Read, Seek, SeekFrom};

/// A bounded reader and seeker over a sub-region of an underlying stream.
/// Equivalent to Python's PartialFile implementation.
pub struct BoundedReader<R> {
    inner: R,
    start: u64,
    size: u64,
    pos: u64,
}

impl<R: Read + Seek> BoundedReader<R> {
    pub fn new(mut inner: R, start: u64, size: u64) -> std::io::Result<Self> {
        inner.seek(SeekFrom::Start(start))?;
        Ok(Self {
            inner,
            start,
            size,
            pos: 0,
        })
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn position(&self) -> u64 {
        self.pos
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read + Seek> Read for BoundedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.size {
            return Ok(0);
        }

        let remaining = self.size - self.pos;
        let max_read = (buf.len() as u64).min(remaining) as usize;
        let n = self.inner.read(&mut buf[..max_read])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl<R: Read + Seek> Seek for BoundedReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos: i128 = match pos {
            SeekFrom::Start(offset) => offset as i128,
            SeekFrom::Current(offset) => (self.pos as i128) + (offset as i128),
            SeekFrom::End(offset) => (self.size as i128) + (offset as i128),
        };

        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Negative offset: {new_pos}"),
            ));
        }

        let new_pos_u64 = new_pos as u64;
        let raw_target = self.start.checked_add(new_pos_u64).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Seek target overflowed u64",
            )
        })?;

        let actual_raw = self.inner.seek(SeekFrom::Start(raw_target))?;
        if actual_raw != raw_target {
            return Err(std::io::Error::other(format!(
                "seek failed: {actual_raw} != {raw_target}"
            )));
        }

        self.pos = new_pos_u64;
        Ok(self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read, Seek, SeekFrom};

    #[test]
    fn test_bounded_reader_read() {
        let data = b"0123456789ABCDEF";
        let cursor = Cursor::new(data.to_vec());
        let mut reader = BoundedReader::new(cursor, 4, 6).unwrap();

        let mut buf = [0u8; 10];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 6);
        assert_eq!(&buf[..6], b"456789");

        // Subsequent read should hit EOF
        let n2 = reader.read(&mut buf).unwrap();
        assert_eq!(n2, 0);
    }

    #[test]
    fn test_bounded_reader_seek() {
        let data = b"0123456789ABCDEF";
        let cursor = Cursor::new(data.to_vec());
        let mut reader = BoundedReader::new(cursor, 2, 8).unwrap();

        // SeekFrom::Start
        assert_eq!(reader.seek(SeekFrom::Start(3)).unwrap(), 3);
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"56");
        assert_eq!(reader.position(), 5);

        // SeekFrom::Current
        assert_eq!(reader.seek(SeekFrom::Current(-2)).unwrap(), 3);
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"56");

        // SeekFrom::End
        assert_eq!(reader.seek(SeekFrom::End(-1)).unwrap(), 7);
        let mut one = [0u8; 1];
        reader.read_exact(&mut one).unwrap();
        assert_eq!(&one, b"9");

        // Negative seek error
        assert!(reader.seek(SeekFrom::Start(0)).is_ok());
        assert!(reader.seek(SeekFrom::Current(-1)).is_err());
    }
}
