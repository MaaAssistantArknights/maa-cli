use std::borrow::Cow;

use anyhow::{Context, anyhow};
use maa_question::prelude::*;
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
    pub fn get_with_desc(&self, description: &'static str) -> anyhow::Result<Option<Cow<'_, str>>> {
        fn some_owned(value: String) -> Option<Cow<'static, str>> {
            Some(Cow::Owned(value))
        }

        match self {
            Secret::None => Ok(None),
            Secret::Prompt => {
                if crate::resolver::is_batch() {
                    anyhow::bail!(
                        "Failed to get {description} from user input: prompt is not available in batch mode"
                    );
                }

                let mut resolver = IoResolver(StdIo::new());
                // HACK: As all inquiries must have a default value, we need to loop until we get a
                // non-empty value
                loop {
                    let answer = resolver
                        .resolve(Inquiry::new(String::new()).with_description(description))
                        .with_context(|| format!("Failed to get {description} from user input"))?;

                    if !answer.is_empty() {
                        break Ok(some_owned(answer));
                    }

                    eprintln!("{description} cannot be empty. Please try again.");
                }
            }
            Secret::Plain(value) => Ok(Some(Cow::Borrowed(value))),
            Secret::Env(name) => std::env::var(name)
                .map(some_owned)
                .with_context(|| format!("Failed to get {description} from environment variable")),
            Secret::Command(cmd) => {
                let Some((program, args)) = cmd.split_first() else {
                    anyhow::bail!("Failed to get {description} from command: command is empty");
                };

                let output = std::process::Command::new(program).args(args).output()?;
                if output.status.success() {
                    let secret = std::str::from_utf8(&output.stdout)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    Ok(some_owned(secret.trim().to_owned()))
                } else {
                    let stderr = String::from_utf8(output.stderr).unwrap_or_default();
                    Err(anyhow!("Failed to execute command {cmd:?}: {stderr}"))
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

    macro_rules! s {
        ($x:literal) => {
            ::std::string::String::from($x)
        };
    }
    macro_rules! svec {
        [$($x:literal),*] => {
            vec![$(s!($x)),*]
        };
    }

    #[test]
    fn deserialize_secret() {
        assert_de_tokens(&Secret::Prompt, &[Token::Bool(true)]);
        assert_de_tokens(&Secret::None, &[Token::Bool(false)]);
        assert_de_tokens(&Secret::Plain(s!("secret")), &[Token::Str("secret")]);
        assert_de_tokens(&Secret::Plain(s!("secret")), &[Token::String("secret")]);
        assert_de_tokens(&Secret::Env(s!("TOKEN")), &[
            Token::Map { len: Some(1) },
            Token::Str("env"),
            Token::Str("TOKEN"),
            Token::MapEnd,
        ]);
        assert_de_tokens(&Secret::Command(svec!["pass", "show"]), &[
            Token::Map { len: Some(1) },
            Token::Str("cmd"),
            Token::Seq { len: Some(2) },
            Token::Str("pass"),
            Token::Str("show"),
            Token::SeqEnd,
            Token::MapEnd,
        ]);
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

    mod resolve {
        use super::*;

        #[test]
        fn none() {
            assert_eq!(Secret::None.get_with_desc("token").unwrap(), None);
        }

        #[test]
        fn plain() {
            assert_eq!(
                Secret::Plain(s!("secret")).get_with_desc("token").unwrap(),
                Some(Cow::Borrowed("secret"))
            );
        }

        #[test]
        fn prompt_rejects_batch_mode() {
            let error = Secret::Prompt.get_with_desc("token").unwrap_err();

            assert_eq!(
                error.to_string(),
                "Failed to get token from user input: prompt is not available in batch mode"
            );
        }

        #[test]
        fn env() {
            const TEST_SECRET_ENV: &str = "MAA_TEST_SECRET";
            assert!(
                Secret::Env(String::from(TEST_SECRET_ENV))
                    .get_with_desc("token")
                    .is_err()
            );

            unsafe { std::env::set_var(TEST_SECRET_ENV, "secret") };
            assert_eq!(
                Secret::Env(String::from(TEST_SECRET_ENV))
                    .get_with_desc("token")
                    .unwrap()
                    .unwrap(),
                "secret"
            );
            unsafe { std::env::remove_var(TEST_SECRET_ENV) };
        }

        mod command {
            use super::*;

            #[test]
            fn success() {
                assert_eq!(
                    Secret::Command(svec!["echo", "secret"])
                        .get_with_desc("token")
                        .unwrap()
                        .unwrap(),
                    "secret"
                );
            }

            #[test]
            fn empty_returns_error() {
                let error = Secret::Command(Vec::new())
                    .get_with_desc("token")
                    .unwrap_err();
                assert_eq!(
                    error.to_string(),
                    "Failed to get token from command: command is empty"
                );
            }

            #[test]
            fn missing_program_returns_not_found() {
                let error: std::io::Error = Secret::Command(svec!["missing-program"])
                    .get_with_desc("token")
                    .unwrap_err()
                    .downcast()
                    .unwrap();
                assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
            }

            #[cfg(unix)]
            #[test]
            fn invalid_utf8_returns_invalid_data_error() {
                let error: std::io::Error = Secret::Command(svec!["sh", "-c", "printf '\\377'"])
                    .get_with_desc("token")
                    .unwrap_err()
                    .downcast()
                    .unwrap();

                assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            }

            #[cfg(unix)]
            #[test]
            fn failure_returns_error() {
                let error = Secret::Command(svec!["sh", "-c", "echo boom >&2; exit 1"])
                    .get_with_desc("token")
                    .unwrap_err();

                assert_eq!(
                    error.to_string(),
                    "Failed to execute command [\"sh\", \"-c\", \"echo boom >&2; exit 1\"]: boom\n"
                );
            }
        }
    }
}
