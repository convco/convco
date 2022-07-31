use super::Command;
use crate::cli::{ChangelogCommand, ReleaseCommand, VersionCommand};

impl Command for ReleaseCommand {
    fn exec(&self, config: crate::conventional::Config) -> Result<(), crate::error::Error> {
        todo!(
            r#"
            - [ ] tag temporary to create changelog
            - [ ] change-version
            - [ ] create changelog
            - [ ] commit"#
        );
    }
}
