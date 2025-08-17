use convco::Config;

mod changelog;
mod check;
mod commit;
mod completions;
mod config;
mod version;

pub(crate) trait Command {
    fn exec(&self, config: Config) -> anyhow::Result<()>;
}
