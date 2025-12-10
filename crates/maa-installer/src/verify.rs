use std::io;

use crate::error::{Error, ErrorKind, Result};

/// A trait representing a verifier that can verify data.
pub trait Verifier: Sized {
    /// Update the verifier with given data.
    fn update(&mut self, data: &[u8]);

    /// Update the verifier with data from a reader.
    ///
    /// # Errors
    ///
    /// If failed to read from the reader, return an IO error.
    fn update_reader<R: io::Read>(&mut self, reader: &mut R) -> Result<()> {
        const READ_BUF_SIZE: usize = 0x2000; // 8KB, which is the same as std::io::copy

        let mut buf = [0; READ_BUF_SIZE];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            self.update(&buf[..n]);
        }
        Ok(())
    }

    /// Finalize and reset the verifier and test if the data is verified.
    ///
    /// # Errors
    ///
    /// If the data is not verified, return an error with kind `Verify`.
    fn verify(&mut self) -> Result<()>;

    /// Convenient method to verify a file.
    ///
    /// # Errors
    ///
    /// If failed to read from the file, return an error with kind `IO`.
    /// If the data is not verified, return an error with kind `Verify`.
    fn verify_file<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        let mut file = std::fs::File::open(path)?;
        self.update_reader(&mut file)?;
        self.verify()
    }
}

impl Verifier for () {
    fn update(&mut self, _data: &[u8]) {}

    fn verify(&mut self) -> Result<()> {
        Ok(())
    }
}

impl<V: Verifier> Verifier for Option<V> {
    fn update(&mut self, data: &[u8]) {
        if let Some(v) = self.as_mut() {
            v.update(data);
        }
    }

    fn update_reader<R: io::Read>(&mut self, reader: &mut R) -> Result<()> {
        if let Some(v) = self.as_mut() {
            v.update_reader(reader)
        } else {
            Ok(())
        }
    }

    fn verify(&mut self) -> Result<()> {
        match self {
            Some(v) => v.verify(),
            None => Ok(()),
        }
    }

    fn verify_file<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        match self {
            Some(v) => v.verify_file(path),
            None => Ok(()),
        }
    }
}

impl<V1: Verifier, V2: Verifier> Verifier for (V1, V2) {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
        self.1.update(data);
    }

    fn verify(&mut self) -> Result<()> {
        self.0.verify()?;
        self.1.verify()?;
        Ok(())
    }
}

/// A size verifier that verifies the total size of input data.
#[derive(Debug, Clone, Copy)]
pub struct SizeVerifier {
    expected: u64,
    current: u64,
}

impl SizeVerifier {
    /// Create a new size verifier with the expected size in bytes.
    pub fn new(expected: u64) -> Self {
        Self {
            expected,
            current: 0,
        }
    }
}

impl Verifier for SizeVerifier {
    fn update(&mut self, data: &[u8]) {
        self.current += data.len() as u64;
    }

    fn verify(&mut self) -> Result<()> {
        let current = std::mem::take(&mut self.current);
        if current == self.expected {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Verify).with_desc(format!(
                "Size mismatch: expected {} bytes, got {} bytes",
                self.expected, current
            )))
        }
    }

    fn verify_file<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        let metadata = std::fs::metadata(path)?;
        if metadata.len() == self.expected {
            self.current = 0; // Reset for reuse
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Verify).with_desc(format!(
                "Size mismatch: expected {} bytes, got {} bytes",
                self.expected,
                metadata.len()
            )))
        }
    }
}

#[cfg(feature = "digest")]
pub mod digest {
    use ::digest::Digest;

    use super::*;

    /// A hash verifier that can be reused by calling reset().
    #[derive(Debug)]
    pub struct DigestVerifier<D: Digest> {
        state: D,
        expected: Vec<u8>,
    }

    impl<D: Digest> DigestVerifier<D> {
        /// Create a new hash verifier with the expected hash.
        ///
        /// This function does not perform any validation on the input bytes.
        ///
        /// Use [`from_slice`] or [`from_hex_str`] to create a verifier from a byte slice or a hex
        /// string.
        pub fn new(bytes: Vec<u8>) -> Self {
            Self {
                state: D::new(),
                expected: bytes,
            }
        }

        fn valid(bytes: &[u8]) -> Result<()> {
            let hash_output_size = <D as Digest>::output_size();
            if bytes.len() != hash_output_size {
                return Err(Error::new(ErrorKind::Verifier).with_desc(format!(
                    "Invalid hash output size: expected {hash_output_size}, got {}",
                    bytes.len()
                )));
            }

            Ok(())
        }

        pub fn from_slice(bytes: &[u8]) -> Result<Self> {
            Self::valid(bytes)?;

            Ok(Self::new(bytes.to_vec()))
        }

        pub fn from_hex_str(hex: &str) -> Result<Self> {
            /// The map of hex characters to their corresponding byte values.
            ///
            /// Instead of using a match statement, this approach is branchless for better
            /// performance.
            static HEX_TABLE: [u8; 256] = {
                let mut t = [0xff; 256];
                let mut i = b'0';
                while i <= b'9' {
                    t[i as usize] = i - b'0';
                    i += 1;
                }
                let mut i = b'a';
                while i <= b'f' {
                    t[i as usize] = i - b'a' + 10;
                    i += 1;
                }
                let mut i = b'A';
                while i <= b'F' {
                    t[i as usize] = i - b'A' + 10;
                    i += 1;
                }
                t
            };

            let len = hex.len();
            if !len.is_multiple_of(2) {
                return Err(Error::new(ErrorKind::Verifier)
                    .with_desc(format!("Invalid hex string length {len}")));
            }

            let bytes = hex
                .as_bytes()
                .chunks(2)
                .enumerate()
                .map(|(i, chunk)| {
                    let hi = HEX_TABLE[chunk[0] as usize];
                    let lo = HEX_TABLE[chunk[1] as usize];
                    if (hi | lo) == 0xff {
                        let (c, idx) = if hi == 0xff {
                            (chunk[0] as char, i * 2)
                        } else {
                            (chunk[1] as char, i * 2 + 1)
                        };
                        return Err(Error::new(ErrorKind::Verifier)
                            .with_desc(format!("Invalid hex character {c} at index and {idx}",)));
                    }
                    Ok(hi << 4 | lo)
                })
                .collect::<Result<Vec<u8>>>()?;

            Self::valid(&bytes)?;

            Ok(Self::new(bytes))
        }
    }

    impl<D: Digest> Verifier for DigestVerifier<D> {
        fn update(&mut self, data: &[u8]) {
            self.state.update(data);
        }

        fn verify(&mut self) -> Result<()> {
            let state = std::mem::replace(&mut self.state, D::new());
            let digest = state.finalize();
            if digest.as_slice() == self.expected {
                Ok(())
            } else {
                Err(Error::new(ErrorKind::Verify).with_desc("digest mismatch"))
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn noop_verifier() {
        let mut verifier = ();
        verifier.update(b"any data");
        assert!(verifier.verify().is_ok());
    }

    #[test]
    fn option_verifier() {
        let mut verifier: Option<()> = None;
        verifier.update(b"test data");
        assert!(verifier.verify().is_ok());

        let mut verifier: Option<()> = Some(());
        verifier.update(b"test data");
        assert!(verifier.verify().is_ok());
    }

    #[test]
    fn composite_verifier() {
        let size_verifier = SizeVerifier::new(12);
        let mut composite = (size_verifier, ());

        composite.update(b"hello ");
        composite.update(b"world");
        composite.update(b"\n");
        assert!(composite.verify().is_ok());
    }

    mod size_verifier {
        use super::*;

        #[test]
        fn base() {
            let mut verifier = SizeVerifier::new(12);
            verifier.update(b"hello ");
            verifier.update(b"world");
            verifier.update(b"\n");
            assert!(verifier.verify().is_ok());

            // Test reuse after successful verification
            verifier.update(b"hello world\n");
            assert!(verifier.verify().is_ok());
        }

        #[test]
        fn mismatch() {
            let mut verifier = SizeVerifier::new(12);
            verifier.update(b"hello world\n");
            verifier.update(b"extra");
            let result = verifier.verify();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), ErrorKind::Verify);
            verifier.update(b"hello");
            let result = verifier.verify();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), ErrorKind::Verify);
        }

        #[test]
        fn option_size_verifier() {
            let mut verifier: Option<SizeVerifier> = Some(SizeVerifier::new(12));
            verifier.update(b"hello world\n");
            assert!(verifier.verify().is_ok());

            let mut verifier: Option<SizeVerifier> = None;
            verifier.update(b"any data");
            assert!(verifier.verify().is_ok());
        }

        #[test]
        fn verify_file() {
            use std::io::Write;

            use tempfile::NamedTempFile;

            let mut verifier = SizeVerifier::new(12);

            // Create a file with known content
            let mut file = NamedTempFile::new().unwrap();
            file.write_all(b"hello world\n").unwrap();
            file.flush().unwrap();
            assert!(verifier.verify_file(file.path()).is_ok());

            // Test with wrong size file
            let mut wrong_file = NamedTempFile::new().unwrap();
            wrong_file.write_all(b"short").unwrap();
            wrong_file.flush().unwrap();
            let result = verifier.verify_file(wrong_file.path());
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), ErrorKind::Verify);
        }
    }

    #[cfg(feature = "digest")]
    #[cfg(test)]
    mod digest_verifier {
        use sha2::Sha256;

        use super::*;
        use crate::{error::ErrorKind, verify::digest::DigestVerifier};

        // This is the hash of "hello world\n" calculated by sha256
        #[rustfmt::skip]
        static HASH: &[u8; 32] = &[
            0xa9, 0x48, 0x90, 0x4f, 0x2f, 0x0f, 0x47, 0x9b,
            0x8f, 0x81, 0x97, 0x69, 0x4b, 0x30, 0x18, 0x4b,
            0x0d, 0x2e, 0xd1, 0xc1, 0xcd, 0x2a, 0x1e, 0xc0,
            0xfb, 0x85, 0xd2, 0x99, 0xa1, 0x92, 0xa4, 0x47,
        ];

        static HASH_STR: &str = "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447";

        #[test]
        fn invalid_hash_length() {
            let result = DigestVerifier::<Sha256>::from_slice(&[1, 2, 3, 4]);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), ErrorKind::Verifier);
        }

        #[test]
        fn hash_from_slice() {
            let mut verifier = DigestVerifier::<Sha256>::from_slice(HASH).unwrap();
            verifier.update(b"hello world\n");
            assert!(verifier.verify().is_ok());
            verifier.update(b"wrong data\n");
            let result = verifier.verify();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), ErrorKind::Verify);

            verifier.update(b"hello ");
            verifier.update(b"world");
            verifier.update(b"\n");
            assert!(verifier.verify().is_ok());
        }

        #[test]
        fn hash_from_hex_str() {
            let mut verifier = DigestVerifier::<Sha256>::from_hex_str(HASH_STR).unwrap();
            verifier.update(b"hello world\n");
            assert!(verifier.verify().is_ok());
            verifier.update(b"wrong data\n");
            let result = verifier.verify();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), ErrorKind::Verify);

            verifier.update(b"hello ");
            verifier.update(b"world");
            verifier.update(b"\n");
            assert!(verifier.verify().is_ok());
        }

        #[test]
        fn hash_verifier_for_file() {
            use std::io::Write;

            use tempfile::NamedTempFile;

            let mut verifier = DigestVerifier::<Sha256>::from_slice(HASH).unwrap();

            let mut file = NamedTempFile::new().unwrap();
            file.write_all(b"hello world\n").unwrap();
            file.flush().unwrap();

            assert!(verifier.verify_file(file.path()).is_ok());

            let mut file = NamedTempFile::new().unwrap();
            file.write_all(b"wrong content\n").unwrap();
            file.flush().unwrap();

            let result = verifier.verify_file(file.path());
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), ErrorKind::Verify);
        }

        #[test]
        fn option_hash_verifier() {
            let mut verifier = Some(DigestVerifier::<Sha256>::from_slice(HASH).unwrap());
            verifier.update(b"hello world\n");
            assert!(verifier.verify().is_ok());
            verifier.update(b"wrong data\n");
            let result = verifier.verify();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), ErrorKind::Verify);

            let mut verifier: Option<DigestVerifier<Sha256>> = None;
            verifier.update(b"any data");
            assert!(verifier.verify().is_ok());
        }

        #[test]
        fn composition_size_hash_verifier() {
            let size_verifier = SizeVerifier::new(12);
            let hash_verifier = DigestVerifier::<Sha256>::from_slice(HASH).unwrap();
            let mut verifier = (size_verifier, hash_verifier);
            verifier.update(b"hello world\n");
            assert!(verifier.verify().is_ok());
            verifier.update(b"wrong data\n");
            let result = verifier.verify();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), ErrorKind::Verify);
            verifier.update(b"wrong hash~\n");
            let result = verifier.verify();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), ErrorKind::Verify);
        }
    }
}
