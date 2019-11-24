use crate::Error;

mod changelog;
mod check;
mod version;

pub(crate) trait Command {
    fn exec(&self) -> Result<(), Error>;
}
