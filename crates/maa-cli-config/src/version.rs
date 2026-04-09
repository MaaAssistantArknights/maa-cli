use serde::Deserialize;

/// Config version for the v2 model defined by this crate.
///
/// `maa-cli` may wrap this with a higher-level versioned parser for v1/v2
/// compatibility, but `Version` itself is a stable public type for the v2
/// format and its schema.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "u64")]
pub enum Version {
    #[default]
    V2,
}

impl Version {
    pub const CURRENT: Self = Self::V2;

    pub const fn get(self) -> u64 {
        match self {
            Self::V2 => 2,
        }
    }
}

impl TryFrom<u64> for Version {
    type Error = String;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value == Self::CURRENT.get() {
            Ok(Self::V2)
        } else {
            Err(format!(
                "unsupported config version `{value}`, expected `{}`",
                Self::CURRENT.get()
            ))
        }
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Version {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Version".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::Version").into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "integer",
            "enum": [2]
        })
    }
}
