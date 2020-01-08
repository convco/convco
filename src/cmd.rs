use crate::Error;

mod changelog;
mod check;
mod commit;
mod version;

pub(crate) trait Command {
    fn exec(&self) -> Result<(), Error>;
}
