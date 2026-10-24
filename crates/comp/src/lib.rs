//! After Effects-style compositions (in progress).

pub mod cli {
    use clap::Subcommand;

    #[derive(Subcommand)]
    pub enum Command {
        /// Print what a composition can contain
        Catalogue,
    }

    pub fn run(command: Command) -> anyhow::Result<()> {
        match command {
            Command::Catalogue => Ok(()),
        }
    }
}
