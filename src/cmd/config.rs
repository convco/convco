use std::io::{stdout, Write};

use crate::{cli::ConfigCommand, cmd::Command, conventional::Config, error::Error};

impl ConfigCommand {
    fn write_yaml(&self, config: &Config, w: impl Write) -> Result<(), Error> {
        Ok(serde_yaml::to_writer(w, config)?)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// This test is really just proving that the yaml (de)serializer for
    /// config is working
    #[test]
    fn test_as_yaml() {
        let config_cmd: ConfigCommand = ConfigCommand { default: true };
        let config: Config = Config::default();
        let mut yaml_config_default = Vec::new();
        config_cmd
            .write_yaml(&config, &mut yaml_config_default)
            .unwrap();
        let yaml_config_default = String::from_utf8(yaml_config_default).unwrap();
        let reparsed_config: Config = serde_yaml::from_str(&yaml_config_default).unwrap();
        assert_eq!(&reparsed_config, &config);
    }
}
