use std::{
    fs,
    io::{stdout, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use convco::{Config, ConvcoError};
use yaml_rt::{JsonPointer, Mapping, Value, YamlDoc, YamlFragment};

use crate::{
    cli::{ConfigAction, ConfigCommand},
    cmd::Command,
};

impl ConfigCommand {
    fn write_yaml(&self, config: &Config, w: impl Write) -> Result<(), ConvcoError> {
        Ok(yaml_rt::to_writer(w, config)?)
    }

    pub(crate) fn exec_with_path(self, config: Config, path: PathBuf) -> anyhow::Result<()> {
        match &self.action {
            None => self.exec(config),
            Some(ConfigAction::Get { key }) => {
                let value = yaml_rt::to_value(config)?;
                let selected = value_at(&value, &parse_key(key)?)?;
                print!("{}", yaml_rt::to_string(selected)?);
                Ok(())
            }
            Some(ConfigAction::Set { .. }) if self.default => {
                anyhow::bail!("--default cannot be used with `config set`");
            }
            Some(ConfigAction::Set { key, value }) => set_value(&path, &config, key, value),
        }
    }
}

impl Command for ConfigCommand {
    fn exec(&self, config: Config) -> anyhow::Result<()> {
        let config = if self.default {
            Config::default()
        } else {
            config
        };
        self.write_yaml(&config, stdout().lock())?;
        Ok(())
    }
}

fn parse_key(key: &str) -> anyhow::Result<Vec<&str>> {
    let parts: Vec<_> = key.split('.').collect();
    if key.is_empty() || parts.iter().any(|part| part.is_empty()) {
        anyhow::bail!("invalid configuration path `{key}`: path segments cannot be empty");
    }
    Ok(parts)
}

fn value_at<'a>(mut value: &'a Value, parts: &[&str]) -> anyhow::Result<&'a Value> {
    for part in parts {
        value = match value {
            Value::Mapping(mapping) => mapping.get(Value::String((*part).to_owned())),
            Value::Sequence(sequence) => {
                let index = part.parse::<usize>().map_err(|_| {
                    anyhow::anyhow!("invalid sequence index `{part}` in configuration path")
                })?;
                sequence.get(index)
            }
            _ => None,
        }
        .ok_or_else(|| {
            anyhow::anyhow!("configuration path `{}` does not exist", parts.join("."))
        })?;
    }
    Ok(value)
}

fn value_at_mut<'a>(mut value: &'a mut Value, parts: &[&str]) -> anyhow::Result<&'a mut Value> {
    for part in parts {
        value = match value {
            Value::Mapping(mapping) => mapping.get_mut(Value::String((*part).to_owned())),
            Value::Sequence(sequence) => {
                let index = part.parse::<usize>().map_err(|_| {
                    anyhow::anyhow!("invalid sequence index `{part}` in configuration path")
                })?;
                sequence.get_mut(index)
            }
            _ => None,
        }
        .ok_or_else(|| {
            anyhow::anyhow!("configuration path `{}` does not exist", parts.join("."))
        })?;
    }
    Ok(value)
}

fn json_pointer(parts: &[&str]) -> anyhow::Result<JsonPointer> {
    let pointer = parts.iter().fold(String::new(), |mut pointer, part| {
        pointer.push('/');
        pointer.push_str(&part.replace('~', "~0").replace('/', "~1"));
        pointer
    });
    Ok(JsonPointer::parse(&pointer)?)
}

fn set_value(path: &Path, config: &Config, key: &str, input: &str) -> anyhow::Result<()> {
    let parts = parse_key(key)?;
    // Traversing the effective model first limits edits to known configuration fields and
    // existing sequence elements.
    let mut desired = yaml_rt::to_value(config)?;
    value_at(&desired, &parts)?;
    let replacement: Value = yaml_rt::from_str(input)
        .map_err(|error| anyhow::anyhow!("invalid YAML value for `{key}`: {error}"))?;
    *value_at_mut(&mut desired, &parts)? = replacement;

    let source = if path.exists() {
        fs::read_to_string(path)?
    } else {
        "{}\n".to_owned()
    };
    let mut document = YamlDoc::parse(&source)?;
    let full_pointer = json_pointer(&parts)?;
    let fragment = YamlFragment::parse(input)
        .map_err(|error| anyhow::anyhow!("invalid YAML value for `{key}`: {error}"))?;

    if document.resolve_pointer(0, &full_pointer).is_ok() {
        document.replace_at(0, &full_pointer, &fragment)?;
    } else {
        let missing = (1..=parts.len())
            .find(|end| {
                document
                    .resolve_pointer(0, &json_pointer(&parts[..*end]).unwrap())
                    .is_err()
            })
            .expect("the complete path was already checked");
        let subtree = minimal_subtree(&desired, &parts, missing)?;
        let subtree_yaml = yaml_rt::to_string(&subtree)?;
        let subtree_fragment = YamlFragment::parse(&subtree_yaml)?;
        document.add_at(0, &json_pointer(&parts[..missing])?, &subtree_fragment)?;
    }

    let rendered = document.to_string();
    yaml_rt::from_str::<Config>(&rendered).map_err(|error| {
        anyhow::anyhow!("setting `{key}` would produce an invalid configuration: {error}")
    })?;
    atomic_write(path, rendered.as_bytes())?;
    Ok(())
}

fn minimal_subtree(desired: &Value, parts: &[&str], missing: usize) -> anyhow::Result<Value> {
    let mut subtree = value_at(desired, parts)?.clone();
    for index in (missing..parts.len()).rev() {
        let parent = value_at(desired, &parts[..index])?;
        subtree = match parent {
            Value::Mapping(_) => {
                let mut mapping = Mapping::new();
                mapping.insert(Value::String(parts[index].to_owned()), subtree);
                Value::Mapping(mapping)
            }
            Value::Sequence(items) => {
                let item_index = parts[index].parse::<usize>().map_err(|_| {
                    anyhow::anyhow!(
                        "invalid sequence index `{}` in configuration path",
                        parts[index]
                    )
                })?;
                let mut prefix = items[..=item_index].to_vec();
                prefix[item_index] = subtree;
                Value::Sequence(prefix)
            }
            _ => anyhow::bail!(
                "configuration path `{}` cannot contain child values",
                parts[..index].join(".")
            ),
        };
    }
    Ok(subtree)
}

fn atomic_write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(".{name}.{}.{sequence}.tmp", std::process::id()));
    let result = (|| {
        fs::write(&temporary, contents)?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_yaml() {
        let config_cmd = ConfigCommand {
            default: true,
            action: None,
        };
        let config = Config::default();
        let mut yaml = Vec::new();
        config_cmd.write_yaml(&config, &mut yaml).unwrap();
        let reparsed: Config = yaml_rt::from_slice(&yaml).unwrap();
        assert_eq!(reparsed, config);
    }

    #[test]
    fn dot_paths_support_mappings_and_sequences() {
        let value = yaml_rt::to_value(Config::default()).unwrap();
        assert_eq!(
            value_at(&value, &parse_key("description.length.min").unwrap())
                .unwrap()
                .as_u64(),
            Some(10)
        );
        assert_eq!(
            value_at(&value, &parse_key("types.0.type").unwrap())
                .unwrap()
                .as_str(),
            Some("feat")
        );
    }

    #[test]
    fn set_preserves_source_and_validates_before_writing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".convco");
        let source = "# local\r\nlineLength: 80 # keep\r\ndescription: {length: {min: 10}}\r\n";
        fs::write(&path, source).unwrap();
        let config = Config::from_path(&path);

        set_value(&path, &config, "lineLength", "100").unwrap();
        set_value(&path, &config, "description.length.max", "72").unwrap();
        let edited = fs::read_to_string(&path).unwrap();
        assert!(edited.starts_with("# local\r\nlineLength: 100 # keep\r\n"));
        assert!(edited.contains("max: 72"));
        assert!(edited.ends_with("\r\n"));

        let before_invalid_edit = edited;
        assert!(set_value(&path, &config, "lineLength", "not-a-number").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), before_invalid_edit);
    }

    #[test]
    fn set_creates_a_minimal_config_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".versionrc");
        set_value(&path, &Config::default(), "description.length.max", "55").unwrap();

        let source = fs::read_to_string(path).unwrap();
        assert!(source.contains("description"));
        assert!(source.contains("max: 55"));
        assert!(!source.contains("min:"));
        assert!(!source.contains("lineLength"));
        yaml_rt::from_str::<Config>(&source).unwrap();
    }

    #[test]
    fn set_replaces_a_complete_sequence_without_touching_surrounding_yaml() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".convco");
        let source = "# before\ntypes: # keep sequence comment\n  - type: feat\n    increment: Minor\n    section: Features\n    hidden: false\nlineLength: 80 # after\n";
        fs::write(&path, source).unwrap();
        let config = Config::from_path(&path);

        set_value(
            &path,
            &config,
            "types",
            "[{type: security, increment: Patch, section: Security, hidden: false}]",
        )
        .unwrap();

        let edited = fs::read_to_string(path).unwrap();
        assert!(edited.starts_with("# before\ntypes: # keep sequence comment\n"));
        assert!(edited.contains("type: security"));
        assert!(edited.ends_with("lineLength: 80 # after\n"));
        assert!(!edited.contains("type: feat"));
    }
}
