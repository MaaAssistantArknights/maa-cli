//! Manifest and asset traits

use std::{borrow::Cow, iter::Map, slice::Iter};

use semver::Version;

use crate::verify::Verifier;

/// A trait for manifests that provides information about the version and assets.
pub trait Manifest {
    /// The type of asset provided by this manifest.
    ///
    /// This must implement the `Asset` trait.
    type Asset<'a>: Asset
    where
        Self: 'a;

    /// Get the version of this manifest.
    ///
    /// Returns the semantic version of the software this manifest represents.
    fn version(&self) -> &Version;

    /// Get the asset for the current platform.
    ///
    /// This method automatically detects the current platform using `std::env::consts`
    /// and returns the appropriate asset if available.
    ///
    /// Returns `None` if no asset is available for the current platform.
    fn asset(&self) -> Option<Self::Asset<'_>>;
}

pub struct MirrorOptions<'a, M: Iterator<Item = Cow<'a, str>>> {
    pub mirrors: M,
    pub max_bytes: u64,
    _marker: std::marker::PhantomData<&'a str>,
}

impl<'a, M: Iterator<Item = Cow<'a, str>>> MirrorOptions<'a, M> {
    pub fn new(mirrors: M, max_bytes: u64) -> Self {
        Self {
            mirrors,
            max_bytes,
            _marker: std::marker::PhantomData,
        }
    }
}

/// A trait for describing an asset that can be downloaded and verified.
///
/// This trait provides information needed to download, mirror, and verify assets.
/// Implementors should provide the asset's URL, optional mirror URLs, and verification
/// information (such as size and digest).
pub trait Asset {
    /// The type of verifier used to verify this asset.
    ///
    /// This can be any type that implements `Verifier` trait.
    type Verifier: Verifier;

    /// Get the name of this asset.
    fn name(&self) -> &str;

    /// Get the primary download URL for this asset.
    ///
    /// Returns a `Cow<str>` to allow both borrowed and owned strings.
    fn url(&self) -> Cow<'_, str>;

    /// Get the mirror options for this asset, if any.
    ///
    /// Returns `None` if no mirrors are available, or `MirrorOptions` containing the mirror
    /// options. Mirrors can be used for faster downloads or as fallbacks if the primary URL
    /// fails.
    fn mirror_opts(&self) -> Option<MirrorOptions<'_, impl Iterator<Item = Cow<'_, str>>>> {
        None::<MirrorOptions<'_, Map<Iter<'_, String>, fn(&String) -> Cow<'_, str>>>>
    }

    /// Create a verifier for this asset.
    ///
    /// The verifier is used to ensure the downloaded file matches the expected
    /// content by checking size, digest, or both.
    ///
    /// # Errors
    ///
    /// Returns an error if the verifier cannot be constructed (e.g., invalid
    /// digest string format).
    fn verifier(&self) -> crate::error::Result<Self::Verifier>;
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::verify::SizeVerifier;

    // Test implementation with minimal verification
    struct SimpleAsset {
        name: String,
        url: String,
        size: u64,
    }

    impl Asset for SimpleAsset {
        type Verifier = SizeVerifier;

        fn name(&self) -> &str {
            &self.name
        }

        fn url(&self) -> Cow<'_, str> {
            Cow::Borrowed(&self.url)
        }

        fn verifier(&self) -> crate::error::Result<Self::Verifier> {
            Ok(SizeVerifier::new(self.size))
        }
    }

    // Test implementation with mirrors and no verification
    struct MirroredAsset {
        name: String,
        url: String,
        size: u64,
        mirrors: Vec<String>,
    }

    impl Asset for MirroredAsset {
        type Verifier = ();

        fn name(&self) -> &str {
            &self.name
        }

        fn url(&self) -> Cow<'_, str> {
            Cow::Borrowed(&self.url)
        }

        fn mirror_opts(&self) -> Option<MirrorOptions<'_, impl Iterator<Item = Cow<'_, str>>>> {
            if self.mirrors.is_empty() {
                None
            } else {
                let mirrors = self.mirrors.iter().map(|m| m.into());
                Some(MirrorOptions::new(
                    mirrors,
                    std::cmp::min(self.size / 2, 1024),
                ))
            }
        }

        fn verifier(&self) -> crate::error::Result<Self::Verifier> {
            Ok(())
        }
    }

    #[test]
    fn simple_asset() {
        let asset = SimpleAsset {
            name: "file.zip".to_string(),
            url: "https://example.com/file.zip".to_string(),
            size: 1024,
        };

        assert_eq!(asset.name(), "file.zip");
        assert_eq!(asset.url(), "https://example.com/file.zip");
        assert!(asset.mirror_opts().is_none());
        assert!(asset.verifier().is_ok());
    }

    #[test]
    fn mirrored_asset() {
        let asset = MirroredAsset {
            name: "file.zip".to_string(),
            url: "https://example.com/file.zip".to_string(),
            size: 1024,
            mirrors: vec![
                "https://mirror1.com/file.zip".to_string(),
                "https://mirror2.com/file.zip".to_string(),
            ],
        };

        assert_eq!(asset.name(), "file.zip");
        assert_eq!(asset.url(), "https://example.com/file.zip");
        let mirror_opts = asset.mirror_opts().unwrap();
        assert_eq!(mirror_opts.max_bytes, std::cmp::min(asset.size / 2, 1024));
        let mirrors = mirror_opts.mirrors.collect::<Vec<_>>();
        assert_eq!(mirrors.len(), 2);
        assert_eq!(mirrors[0], "https://mirror1.com/file.zip");
        assert_eq!(mirrors[1], "https://mirror2.com/file.zip");
    }

    #[test]
    fn asset_with_no_mirrors() {
        let asset = MirroredAsset {
            name: "file.zip".to_string(),
            url: "https://example.com/file.zip".to_string(),
            size: 4096,
            mirrors: vec![],
        };

        assert!(asset.mirror_opts().is_none());
    }

    #[cfg(feature = "digest")]
    #[test]
    fn asset_with_digest() {
        use sha2::Sha256;

        use crate::verify::digest::DigestVerifier;

        struct DigestAsset {
            name: String,
            url: String,
            size: u64,
            sha256: String,
        }

        impl Asset for DigestAsset {
            type Verifier = (SizeVerifier, DigestVerifier<Sha256>);

            fn name(&self) -> &str {
                &self.name
            }

            fn url(&self) -> Cow<'_, str> {
                Cow::Borrowed(&self.url)
            }

            fn verifier(&self) -> crate::error::Result<Self::Verifier> {
                let size_verifier = SizeVerifier::new(self.size);
                let digest_verifier = DigestVerifier::<Sha256>::from_hex_str(&self.sha256)?;
                Ok((size_verifier, digest_verifier))
            }
        }

        let asset = DigestAsset {
            name: "file.zip".to_string(),
            url: "https://example.com/file.zip".to_string(),
            size: 12,
            sha256: "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447".to_string(),
        };

        assert_eq!(asset.name(), "file.zip");
        assert_eq!(asset.url(), "https://example.com/file.zip");
        assert!(asset.verifier().is_ok());

        // Test with invalid hash
        let bad_asset = DigestAsset {
            name: "file.zip".to_string(),
            url: "https://example.com/file.zip".to_string(),
            size: 12,
            sha256: "invalid".to_string(),
        };

        assert!(bad_asset.verifier().is_err());
    }

    #[test]
    fn asset_with_optional_verifier() {
        struct OptionalVerifyAsset {
            name: String,
            url: String,
            size: Option<u64>,
        }

        impl Asset for OptionalVerifyAsset {
            type Verifier = Option<SizeVerifier>;

            fn name(&self) -> &str {
                &self.name
            }

            fn url(&self) -> Cow<'_, str> {
                Cow::Borrowed(&self.url)
            }

            fn verifier(&self) -> crate::error::Result<Self::Verifier> {
                Ok(self.size.map(SizeVerifier::new))
            }
        }

        let asset_with_size = OptionalVerifyAsset {
            name: "file.zip".to_string(),
            url: "https://example.com/file.zip".to_string(),
            size: Some(1024),
        };
        assert!(asset_with_size.verifier().unwrap().is_some());

        let asset_without_size = OptionalVerifyAsset {
            name: "file.zip".to_string(),
            url: "https://example.com/file.zip".to_string(),
            size: None,
        };
        assert!(asset_without_size.verifier().unwrap().is_none());
    }
}
