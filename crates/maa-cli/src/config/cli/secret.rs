use std::borrow::Cow;

use anyhow::Context;
use maa_value::userinput::{Input, UserInput};
use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Default)]
pub enum Secret {
    #[default]
    None,
    Prompt,
    Plain(String),
    Env(String),
    Command(Vec<String>),
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        enum SecretField {
            Cmd,
            Env,
        }

        impl<'de> serde::Deserialize<'de> for SecretField {
            fn deserialize<D>(deserializer: D) -> Result<SecretField, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                struct SecretFieldVisitor;

                impl serde::de::Visitor<'_> for SecretFieldVisitor {
                    type Value = SecretField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`cmd` or `env`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "cmd" => Ok(SecretField::Cmd),
                            "env" => Ok(SecretField::Env),
                            _ => Err(serde::de::Error::unknown_field(value, &["cmd", "env"])),
                        }
                    }
                }

                deserializer.deserialize_identifier(SecretFieldVisitor)
            }
        }

        struct SecretVisitor;

        impl<'de> serde::de::Visitor<'de> for SecretVisitor {
            type Value = Secret;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid secret, which must be a bool, string, or map")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value {
                    Ok(Secret::Prompt)
                } else {
                    Ok(Secret::None)
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Secret::Plain(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Secret::Plain(value))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let secret = match map.next_key::<SecretField>()? {
                    Some(SecretField::Cmd) => Secret::Command(map.next_value()?),
                    Some(SecretField::Env) => Secret::Env(map.next_value()?),
                    None => return Err(serde::de::Error::custom("empty map")),
                };

                if map.next_key::<SecretField>()?.is_some() {
                    return Err(serde::de::Error::custom(
                        "expected a map with a single key-value pair",
                    ));
                }

                Ok(secret)
            }
        }

        deserializer.deserialize_any(SecretVisitor)
    }
}

impl Secret {
    pub fn get_with_description(
        &self,
        description: impl Into<Cow<'static, str>>,
    ) -> anyhow::Result<Option<Cow<'_, str>>> {
        let description = description.into();

        match self {
            Secret::None => Ok(None),
            Secret::Prompt => Input::<String>::new(None)
                .with_description(description.clone())
                .value()
                .map(Cow::Owned)
                .map(Some)
                .with_context(|| format!("Failed to get {} from user input", description.as_ref())),
            Secret::Plain(value) => Ok(Some(Cow::Borrowed(value))),
            Secret::Env(name) => std::env::var(name)
                .map(Cow::Owned)
                .map(Some)
                .with_context(|| {
                    format!(
                        "Failed to get {} from environment variable",
                        description.as_ref()
                    )
                }),
            Secret::Command(cmd) => {
                let Some(program) = cmd.first() else {
                    anyhow::bail!(
                        "Failed to get {} from command: command is empty",
                        description.as_ref()
                    );
                };

                let output = std::process::Command::new(program)
                    .args(&cmd[1..])
                    .output()?;
                if output.status.success() {
                    let secret = std::str::from_utf8(&output.stdout)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    Ok(Some(Cow::Owned(secret.trim().to_owned())))
                } else {
                    let stderr = String::from_utf8(output.stderr).unwrap_or_default();
                    Err(anyhow::anyhow!(
                        "Failed to execute command {:?}: {}",
                        cmd,
                        stderr
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_test::{Token, assert_de_tokens, assert_de_tokens_error};

    use super::*;

    #[test]
    fn deserialize_secret() {
        assert_de_tokens(&Secret::Prompt, &[Token::Bool(true)]);
        assert_de_tokens(&Secret::None, &[Token::Bool(false)]);
        assert_de_tokens(&Secret::Plain(String::from("secret")), &[Token::Str(
            "secret",
        )]);
        assert_de_tokens(&Secret::Env(String::from("TOKEN")), &[
            Token::Map { len: Some(1) },
            Token::Str("env"),
            Token::Str("TOKEN"),
            Token::MapEnd,
        ]);
        assert_de_tokens(
            &Secret::Command(vec![String::from("pass"), String::from("show")]),
            &[
                Token::Map { len: Some(1) },
                Token::Str("cmd"),
                Token::Seq { len: Some(2) },
                Token::Str("pass"),
                Token::Str("show"),
                Token::SeqEnd,
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn deserialize_secret_error() {
        assert_de_tokens_error::<Secret>(
            &[Token::Map { len: Some(0) }, Token::MapEnd],
            "empty map",
        );
        assert_de_tokens_error::<Secret>(
            &[Token::Map { len: Some(1) }, Token::Str("foo")],
            "unknown field `foo`, expected `cmd` or `env`",
        );
        assert_de_tokens_error::<Secret>(
            &[
                Token::Map { len: Some(2) },
                Token::Str("cmd"),
                Token::Seq { len: Some(2) },
                Token::Str("get"),
                Token::Str("secret"),
                Token::SeqEnd,
                Token::Str("env"),
            ],
            "expected a map with a single key-value pair",
        );
        assert_de_tokens_error::<Secret>(
            &[Token::I64(0)],
            "invalid type: integer `0`, \
            expected a valid secret, which must be a bool, string, or map",
        );
    }

    #[test]
    fn empty_command_returns_error() {
        let error = Secret::Command(Vec::new())
            .get_with_description("token")
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Failed to get token from command: command is empty"
        );
    }

    #[test]
    fn resolve_secret() {
        assert_eq!(Secret::None.get_with_description("token").unwrap(), None);

        assert_eq!(
            Secret::Plain(String::from("secret"))
                .get_with_description("token")
                .unwrap(),
            Some(Cow::Borrowed("secret"))
        );

        assert!(
            Secret::Env(String::from("MMA_TEST_SECRET"))
                .get_with_description("token")
                .is_err()
        );

        unsafe { std::env::set_var("MMA_TEST_SECRET", "secret") };
        assert_eq!(
            Secret::Env(String::from("MMA_TEST_SECRET"))
                .get_with_description("token")
                .unwrap()
                .unwrap(),
            "secret"
        );
        unsafe { std::env::remove_var("MMA_TEST_SECRET") };

        assert_eq!(
            Secret::Command(vec![String::from("echo"), String::from("secret")])
                .get_with_description("token")
                .unwrap()
                .unwrap(),
            "secret"
        );
    }
}
