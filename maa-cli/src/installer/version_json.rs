use semver::Version;
use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct VersionJSON<D> {
    version: Version,
    details: D,
}

#[derive(Deserialize)]
struct VersionJSONHelper<D> {
    version: String,
    details: D,
}

impl<'de, A: Deserialize<'de>> Deserialize<'de> for VersionJSON<A> {
    fn deserialize<D>(deserializer: D) -> Result<VersionJSON<A>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
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
            println!(
                "Found newer {} version: v{} (current: v{})",
                name, version, current_version
            );
            Ok(true)
        } else {
            println!("Up to date: {} v{}.", name, current_version);
            Ok(false)
        }
    }

    pub fn details(&self) -> &D {
        &self.details
    }
}
