use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub struct Writer {
    pub file: Option<File>,
    pub document_index: usize,
}

impl Writer {
    pub fn new(path: Option<PathBuf>) -> Result<Self> {
        let file = match path {
            Some(p) => Some(File::create(p)?),
            None => None,
        };
        Ok(Self {
            file,
            document_index: 1,
        })
    }

    pub fn write(&mut self, content: &str) -> Result<()> {
        match &mut self.file {
            Some(f) => {
                writeln!(f, "{}", content)?;
                Ok(())
            }
            None => {
                println!("{}", content);
                Ok(())
            }
        }
    }
}
