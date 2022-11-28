use clap::Parser;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct TakinaArgs {
    #[arg(short = 'f', long)]
    /// config file path (default to ./takina.toml)
    pub config: Option<String>,
    #[arg(short, long)]
    /// check if config can be parsed without errors
    pub check: bool,
}
