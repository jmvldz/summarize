use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Default,
    Cxml,
    Markdown,
}
