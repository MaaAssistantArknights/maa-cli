use semver::Version;
use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct VersionJSON<D> {
    version: Version,
    details: D,
}

impl<'de, A: Deserialize<'de>> Deserialize<'de> for VersionJSON<A> {
    fn deserialize<D>(deserializer: D) -> Result<VersionJSON<A>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct VersionJSONHelper<D> {
            version: String,
            details: D,
        }

        let helper = VersionJSONHelper::deserialize(deserializer)?;
        let version = if helper.version.starts_with('v') {
            Version::parse(&helper.version[1..])
        } else {
            Version::parse(&helper.version)
        }
        .map_err(serde::de::Error::custom)?;

        Ok(VersionJSON {
            version,
            details: helper.details,
        })
    }
}

impl<D> VersionJSON<D> {
    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn can_update(&self, name: &str, current_version: &Version) -> Result<bool, semver::Error> {
        let version = self.version();
        if version > current_version {
            println!("Found newer {name} version: v{version} (current: v{current_version})");
            Ok(true)
        } else {
            println!("Up to date: {name} v{current_version}.");
            Ok(false)
        }
    }

    pub fn details(&self) -> &D {
        &self.details
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_can_update() {
        fn can_update(remote: &str, current: &str, expected: bool) {
            let version_json = VersionJSON {
                version: Version::parse(remote).unwrap(),
                details: (),
            };

            let current_version = Version::parse(current).unwrap();
            assert_eq!(
                version_json.can_update("test", &current_version).unwrap(),
                expected
            );
        }

        can_update("0.1.0", "0.0.9", true);
        can_update("0.1.0", "0.1.0", false);
        can_update("0.1.0", "0.1.1", false);

        can_update("0.1.0", "0.1.0-beta", true);
        can_update("0.1.0", "0.1.1-beta", false);

        can_update("0.1.0", "0.1.0-alpha", true);
        can_update("0.1.0", "0.1.1-alpha", false);

        can_update("0.1.0-beta", "0.1.0", false);
        can_update("0.1.1-beta", "0.1.0", true);

        can_update("0.1.0-beta.1", "0.1.0-beta", true);
        can_update("0.1.0-beta.2", "0.1.0-beta.1", true);

        can_update("0.1.0-beta", "0.1.0-alpha", true);
        can_update("0.1.0-beta", "0.1.0-alpha.1", true);
        can_update("0.1.0-beta.1.alpha", "0.1.0-beta.1", true);
        can_update("0.1.0-beta.1.alpha.1", "0.1.0-beta.1", true);
        can_update("0.1.0-beta.1.alpha.2", "0.1.0-beta.1.alpha.1", true);
        can_update("0.1.0-beta.2.alpha.1", "0.1.0-beta.1.alpha.2", true);
        can_update("0.1.0-alpha.1+sha.1da7b3d", "0.1.0-alpha.1", true);
    }
}
