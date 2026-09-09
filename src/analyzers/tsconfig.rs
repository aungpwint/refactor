use crate::context::RepoContext;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

pub type PathAliases = HashMap<String, Vec<String>>;

pub fn read_path_aliases(ctx: &RepoContext) -> PathAliases {
    let Some(tsconfig) = ctx.tsconfig_path() else {
        return PathAliases::new();
    };
    read_aliases_from_file(&tsconfig).unwrap_or_default()
}

fn read_aliases_from_file(path: &Path) -> Option<PathAliases> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_aliases(&content)
}

fn parse_aliases(content: &str) -> Option<PathAliases> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TsConfig {
        compiler_options: Option<CompilerOptions>,
    }
    #[derive(Deserialize)]
    struct CompilerOptions {
        #[allow(dead_code)]
        base_url: Option<String>,
        paths: Option<PathMap>,
    }
    struct PathMap(Vec<(String, Vec<String>)>);

    impl<'de> serde::Deserialize<'de> for PathMap {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let map = HashMap::<String, Vec<String>>::deserialize(deserializer)?;
            Ok(PathMap(map.into_iter().collect()))
        }
    }

    let tsconfig: TsConfig = serde_json::from_str(content).ok()?;
    let compiler = tsconfig.compiler_options?;
    let paths = compiler.paths?;
    Some(paths.0.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tsconfig_aliases() {
        let content = r##"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "@/*": ["./app/resources/js/*"],
                    "#/*": ["src/*"]
                }
            }
        }"##;
        let aliases = parse_aliases(content).unwrap();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases["@/*"], vec!["./app/resources/js/*"]);
        assert_eq!(aliases["#/*"], vec!["src/*"]);
    }

    #[test]
    fn handles_missing_paths() {
        let content = r##"{"compilerOptions": {"strict": true}}"##;
        assert!(parse_aliases(content).is_none());
    }
}
