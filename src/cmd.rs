use crate::{conventional::Config, Error};

mod changelog;
mod check;
mod commit;
mod version;

pub(crate) trait Command {
    fn exec(&self, config: Config) -> Result<(), Error>;
}
