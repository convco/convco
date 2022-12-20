use crate::conventional::Config;

mod changelog;
mod check;
mod commit;
mod config;
mod version;

pub(crate) trait Command {
    fn exec(&self, config: Config) -> anyhow::Result<()>;
}
