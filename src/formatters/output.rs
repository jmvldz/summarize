use anyhow::Result;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::Path;

use super::Writer;
use crate::models::OutputFormat;

// Maps file extensions to language names for markdown formatting
lazy_static! {
    static ref EXT_TO_LANG: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("py", "python");
        m.insert("c", "c");
        m.insert("cpp", "cpp");
        m.insert("h", "c");
        m.insert("hpp", "cpp");
        m.insert("java", "java");
        m.insert("js", "javascript");
        m.insert("ts", "typescript");
        m.insert("html", "html");
        m.insert("css", "css");
        m.insert("xml", "xml");
        m.insert("json", "json");
        m.insert("yaml", "yaml");
        m.insert("yml", "yaml");
        m.insert("sh", "bash");
        m.insert("rb", "ruby");
        m.insert("rs", "rust");
        m.insert("go", "go");
        m.insert("md", "markdown");
        m.insert("toml", "toml");
        m
    };
}

pub fn add_line_numbers(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let padding = lines.len().to_string().len();

    lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:padding$}  {}", i + 1, line, padding = padding))
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn print_path(
    writer: &mut Writer,
    path: &Path,
    content: &str,
    format: &OutputFormat,
    line_numbers: bool,
) -> Result<()> {
    match format {
        OutputFormat::Cxml => print_as_xml(writer, path, content, line_numbers),
        OutputFormat::Markdown => print_as_markdown(writer, path, content, line_numbers),
        OutputFormat::Default => print_default(writer, path, content, line_numbers),
    }
}

pub fn print_default(
    writer: &mut Writer,
    path: &Path,
    content: &str,
    line_numbers: bool,
) -> Result<()> {
    writer.write(&path.to_string_lossy())?;
    writer.write("---")?;

    let content_to_write = if line_numbers {
        add_line_numbers(content)
    } else {
        content.to_string()
    };

    writer.write(&content_to_write)?;
    writer.write("")?;
    writer.write("---")?;
    Ok(())
}

pub fn print_as_xml(
    writer: &mut Writer,
    path: &Path,
    content: &str,
    line_numbers: bool,
) -> Result<()> {
    writer.write(&format!(r#"<document index="{}">"#, writer.document_index))?;
    writer.write(&format!(r#"<source>{}</source>"#, path.to_string_lossy()))?;
    writer.write("<document_content>")?;

    let content_to_write = if line_numbers {
        add_line_numbers(content)
    } else {
        content.to_string()
    };

    writer.write(&content_to_write)?;
    writer.write("</document_content>")?;
    writer.write("</document>")?;

    writer.document_index += 1;
    Ok(())
}

pub fn print_as_markdown(
    writer: &mut Writer,
    path: &Path,
    content: &str,
    line_numbers: bool,
) -> Result<()> {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    let lang = EXT_TO_LANG.get(extension).copied().unwrap_or("");

    // Figure out how many backticks to use
    let mut backticks = "```".to_string();
    while content.contains(&backticks) {
        backticks.push('`');
    }

    writer.write(&path.to_string_lossy())?;
    writer.write(&format!("{}{}", backticks, lang))?;

    let content_to_write = if line_numbers {
        add_line_numbers(content)
    } else {
        content.to_string()
    };

    writer.write(&content_to_write)?;
    writer.write(&backticks)?;
    Ok(())
}
