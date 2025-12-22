use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::cli::normalize_url;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_api_url")]
    api_url: String,
    /// Check interval in seconds (0 to disable caching)
    #[serde(default = "default_check_interval")]
    check_interval: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            check_interval: default_check_interval(),
        }
    }
}

fn default_api_url() -> String {
    "https://api.maa.plus/MaaAssistantArknights/api".to_string()
}

fn default_check_interval() -> u64 {
    600 // 10 min in seconds
}

impl Config {
    const RESOURCE_FILES: [&[&str]; 6] = [
        &["tasks.json"],
        &["platform_diff", "iOS", "resource", "tasks.json"],
        &["global", "YoStarEN", "resource", "tasks.json"],
        &["global", "YoStarJP", "resource", "tasks.json"],
        &["global", "YoStarKR", "resource", "tasks.json"],
        &["global", "txwy", "resource", "tasks.json"],
    ];

    pub fn api_url(&self) -> &str {
        normalize_url(&self.api_url)
    }

    pub fn check_interval(&self) -> Option<std::time::Duration> {
        if self.check_interval == 0 {
            None
        } else {
            Some(std::time::Duration::from_secs(self.check_interval))
        }
    }

    pub fn resource_files(&self) -> impl Iterator<Item = PathBuf> {
        let resource_dir = maa_dirs::hot_update_resource().to_path_buf();
        Self::RESOURCE_FILES
            .iter()
            .map(move |path| resource_dir.clone().join_iter(path.iter()))
    }

    pub fn activity_url(&self) -> String {
        Url(self.api_url().to_owned())
            .join_iter(["gui", "StageActivityV2.json"].iter())
            .0
    }

    pub fn resource_urls(&self) -> impl Iterator<Item = String> {
        let resource_url = format!("{}/resource", self.api_url());
        Self::RESOURCE_FILES
            .iter()
            .map(move |path| Url(resource_url.clone()).join_iter(path.iter()).0)
    }
}

trait JoinIter<C> {
    fn join_iter(self, iter: impl Iterator<Item = C>) -> Self;
}

impl<P: AsRef<Path>> JoinIter<P> for PathBuf {
    fn join_iter(mut self, iter: impl Iterator<Item = P>) -> PathBuf {
        for path in iter {
            self.push(path);
        }
        self
    }
}

struct Url(String);

impl<P: AsRef<str>> JoinIter<P> for Url {
    fn join_iter(self, iter: impl Iterator<Item = P>) -> Url {
        let mut s = self.0;
        for comp in iter {
            s.push('/');
            s.push_str(comp.as_ref());
        }
        Url(s)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod tests {
    use super::*;

    pub fn example_config() -> Config {
        Config {
            api_url: "https://github.com/MaaAssistantArknights/MaaRelease/raw/main/MaaAssistantArknights/api".to_string(),
            check_interval: 3600, // 1 hour
        }
    }

    #[test]
    fn default() {
        let config = Config::default();
        assert_eq!(config, Config {
            api_url: default_api_url(),
            check_interval: default_check_interval(),
        });
    }

    mod serde {
        use serde_test::{Token, assert_de_tokens};

        use super::*;

        #[test]
        fn deserialize_config() {
            // Default config
            assert_de_tokens(&Config::default(), &[
                Token::Map { len: Some(0) },
                Token::MapEnd,
            ]);

            // Custom api_url
            assert_de_tokens(
                &Config {
                    api_url: "https://custom.api.com".to_string(),
                    check_interval: default_check_interval(),
                },
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("api_url"),
                    Token::Str("https://custom.api.com"),
                    Token::MapEnd,
                ],
            );

            // Custom check_interval
            assert_de_tokens(
                &Config {
                    api_url: default_api_url(),
                    check_interval: 3600, // 1 hour
                },
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("check_interval"),
                    Token::U64(3600),
                    Token::MapEnd,
                ],
            );

            // Both custom
            assert_de_tokens(
                &Config {
                    api_url: "https://custom.api.com".to_string(),
                    check_interval: 0, // Disable caching
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("api_url"),
                    Token::Str("https://custom.api.com"),
                    Token::Str("check_interval"),
                    Token::U64(0),
                    Token::MapEnd,
                ],
            );
        }
    }

    mod methods {
        use super::*;

        #[test]
        fn resource_files() {
            let config = Config::default();
            let files: Vec<PathBuf> = config.resource_files().collect();

            assert_eq!(files.len(), 6);

            let resource_dir = maa_dirs::cache().join("resource");
            assert_eq!(files[0], resource_dir.join("tasks.json"));
            assert_eq!(
                files[1],
                resource_dir
                    .join("platform_diff")
                    .join("iOS")
                    .join("resource")
                    .join("tasks.json")
            );
            assert_eq!(
                files[2],
                resource_dir
                    .join("global")
                    .join("YoStarEN")
                    .join("resource")
                    .join("tasks.json")
            );
            assert_eq!(
                files[3],
                resource_dir
                    .join("global")
                    .join("YoStarJP")
                    .join("resource")
                    .join("tasks.json")
            );
            assert_eq!(
                files[4],
                resource_dir
                    .join("global")
                    .join("YoStarKR")
                    .join("resource")
                    .join("tasks.json")
            );
            assert_eq!(
                files[5],
                resource_dir
                    .join("global")
                    .join("txwy")
                    .join("resource")
                    .join("tasks.json")
            );
        }

        #[test]
        fn activity_url() {
            let config = Config {
                api_url: "https://api.example.com".to_string(),
                ..Default::default()
            };
            assert_eq!(
                config.activity_url(),
                "https://api.example.com/gui/StageActivityV2.json"
            );

            // Test with default
            let default_config = Config::default();
            assert_eq!(
                default_config.activity_url(),
                "https://api.maa.plus/MaaAssistantArknights/api/gui/StageActivityV2.json"
            );
        }

        #[test]
        fn resource_file_paths() {
            let config = Config {
                api_url: "https://api.example.com".to_string(),
                ..Default::default()
            };

            let paths: Vec<String> = config.resource_urls().collect();

            assert_eq!(paths.len(), 6);
            assert_eq!(paths[0], "https://api.example.com/resource/tasks.json");
            assert_eq!(
                paths[1],
                "https://api.example.com/resource/platform_diff/iOS/resource/tasks.json"
            );
            assert_eq!(
                paths[2],
                "https://api.example.com/resource/global/YoStarEN/resource/tasks.json"
            );
            assert_eq!(
                paths[3],
                "https://api.example.com/resource/global/YoStarJP/resource/tasks.json"
            );
            assert_eq!(
                paths[4],
                "https://api.example.com/resource/global/YoStarKR/resource/tasks.json"
            );
            assert_eq!(
                paths[5],
                "https://api.example.com/resource/global/txwy/resource/tasks.json"
            );
        }

        #[test]
        fn check_interval() {
            let config = Config::default();
            assert_eq!(
                config.check_interval(),
                Some(std::time::Duration::from_secs(600))
            );

            let config = Config {
                check_interval: 3600,
                ..Default::default()
            };
            assert_eq!(
                config.check_interval(),
                Some(std::time::Duration::from_secs(3600))
            );

            let config = Config {
                check_interval: 0,
                ..Default::default()
            };
            assert_eq!(config.check_interval(), None);
        }

        #[test]
        fn api_url_trailing_slash() {
            // Test that URLs work with or without trailing slash
            let config_with_slash = Config {
                api_url: "https://api.example.com/".to_string(),
                ..Default::default()
            };
            let config_without_slash = Config {
                api_url: "https://api.example.com".to_string(),
                ..Default::default()
            };

            assert_eq!(
                config_with_slash.activity_url(),
                "https://api.example.com/gui/StageActivityV2.json"
            );
            assert_eq!(
                config_without_slash.activity_url(),
                "https://api.example.com/gui/StageActivityV2.json"
            );
        }
    }

    mod traits {
        use super::*;

        #[test]
        fn pathbuf_join_iter() {
            let base = PathBuf::from("/base");
            let segments = ["dir1", "dir2", "file.txt"];
            let result = base.join_iter(segments.iter());
            assert_eq!(result, PathBuf::from("/base/dir1/dir2/file.txt"));

            // Test with empty iterator
            let empty: Vec<&str> = vec![];
            let result = PathBuf::from("/base").join_iter(empty.iter());
            assert_eq!(result, PathBuf::from("/base"));
        }

        #[test]
        fn url_join_iter() {
            let base = Url("https://example.com".to_string());
            let segments = ["api", "v1", "resource"];
            let result = base.join_iter(segments.iter());
            assert_eq!(result.0, "https://example.com/api/v1/resource");

            // Test with empty iterator
            let empty: Vec<&str> = vec![];
            let result = Url("https://example.com".to_string()).join_iter(empty.iter());
            assert_eq!(result.0, "https://example.com");
        }
    }
}
