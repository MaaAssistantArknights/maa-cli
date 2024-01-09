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

    pub fn can_update(&self, name: &str, ver_current: &Version) -> Result<bool, semver::Error> {
        let ver_remote = self.version();
        if ver_remote > ver_current {
            printlnfl!(
                "found-newer-version",
                name = name,
                new = ver_remote.to_string(),
                old = ver_current.to_string()
            );
            Ok(true)
        } else {
            printlnfl!(
                "update-to-date",
                name = name,
                version = ver_current.to_string()
            );
            Ok(false)
        }
    }

    pub fn details(self) -> D {
        self.details
    }
}
