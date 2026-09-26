use clap::Parser as ClapParser;
use console::{style, truncate_str};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::{collections::HashSet, env, error::Error, fs, path::Path, process::ExitCode};

#[derive(ClapParser)]
#[command(
    about = "Search RFC titles and sections",
    after_help = "All query words must match within a section or its RFC title.
With no query, list RFCs. Search ignores case."
)]
struct Args {
    /// Status to include, or 'all' for every status
    #[arg(long, default_value = "accepted")]
    status: String,
    /// Words to search for
    query: Vec<String>,
}

struct Section {
    heading: String,
    anchor: String,
    line: usize,
    text: String,
}

struct Rfc {
    title: String,
    status: String,
    sections: Vec<Section>,
}

fn parse(source: &str, filename: &str) -> Rfc {
    let mut rfc = Rfc {
        title: filename.into(),
        status: "Unknown".into(),
        sections: Vec::new(),
    };
    let mut headings = Vec::new();
    let mut current = None;
    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some((level, range.start, String::new()))
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, heading)) = &mut current {
                    heading.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, start, heading)) = current.take() {
                    if level == HeadingLevel::H1 && rfc.title == filename {
                        rfc.title = heading.clone();
                    }
                    headings.push((start, heading));
                }
            }
            _ => {}
        }
    }
    if headings.first().is_none_or(|(start, _)| *start > 0) {
        headings.insert(0, (0, filename.into()));
    }
    let mut anchors = HashSet::new();
    for (index, (start, heading)) in headings.iter().enumerate() {
        let end = headings
            .get(index + 1)
            .map_or(source.len(), |(start, _)| *start);
        let base: String = heading
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || matches!(c, '_' | '-'))
            .map(|c| if c.is_whitespace() { '-' } else { c })
            .collect();
        let mut anchor = base.clone();
        let mut suffix = 1;
        while !anchors.insert(anchor.clone()) {
            anchor = format!("{base}-{suffix}");
            suffix += 1;
        }
        rfc.sections.push(Section {
            heading: heading.clone(),
            anchor,
            line: source[..*start].lines().count() + 1,
            text: source[*start..end].into(),
        });
    }
    if let Some(status) = source
        .lines()
        .find_map(|line| line.trim().strip_prefix("- **Status:**"))
    {
        rfc.status = status.trim().into();
    }
    rfc
}

fn search(directory: &Path, status: &str, terms: &[String]) -> Result<String, Box<dyn Error>> {
    let mut files = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    files.sort_by_key(|entry| entry.file_name());
    let mut output = String::new();
    let width = usize::from(console::Term::stdout().size().1.saturating_sub(4).max(20));
    for entry in files {
        let filename = entry.file_name().to_string_lossy().into_owned();
        let Some((number, name)) = filename.split_once('-') else {
            continue;
        };
        if number.len() != 4
            || !number.bytes().all(|c| c.is_ascii_digit())
            || !name.ends_with(".md")
            || !entry.file_type()?.is_file()
        {
            continue;
        }
        let rfc = parse(&fs::read_to_string(entry.path())?, &filename);
        if !status.eq_ignore_ascii_case("all") && !rfc.status.eq_ignore_ascii_case(status) {
            continue;
        }
        let matches: Vec<_> = rfc
            .sections
            .iter()
            .filter(|section| {
                let text = format!("{}\n{}", rfc.title, section.text).to_lowercase();
                terms.iter().all(|term| text.contains(term))
            })
            .collect();
        if matches.is_empty() {
            continue;
        }
        output.push_str(&format!(
            "{}  {}\n",
            style(&rfc.title).cyan().bold(),
            style(&rfc.status).dim()
        ));
        for section in matches
            .into_iter()
            .take(if terms.is_empty() { 1 } else { usize::MAX })
        {
            if !terms.is_empty() {
                output.push_str(&format!("  {}\n", style(&section.heading).bold()));
            }
            if !terms.is_empty() {
                let excerpt = section
                    .text
                    .lines()
                    .skip(1)
                    .find(|line| {
                        !line.trim().is_empty()
                            && terms.iter().any(|term| line.to_lowercase().contains(term))
                    })
                    .unwrap_or("")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                if !excerpt.is_empty() {
                    output.push_str(&format!("  {}\n", truncate_str(&excerpt, width, "...")));
                }
            }
            output.push_str(&format!(
                "  {}\n",
                style(format!(
                    "[rfcs/{filename}:{}](rfcs/{filename}#{})",
                    section.line, section.anchor
                ))
                .dim()
            ));
        }
        output.push('\n');
    }
    if output.is_empty() {
        output.push_str("No matching RFCs.\n");
    }
    Ok(output)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let root = env::var_os("DEVENV_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let terms = args
        .query
        .iter()
        .flat_map(|s| s.split_whitespace().map(str::to_lowercase))
        .collect::<Vec<_>>();
    print!("{}", search(&root.join("rfcs"), &args.status, &terms)?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_skips_headings_inside_fences() {
        let rfc = parse(
            "# RFC 0001: Storage\r\n- **Status:** Accepted\r\n## Design\r\nRedis\r\n~~~~\r\n# comment\r\n~~~\r\n~~~~\r\n## Design\r\nPostgres",
            "0001-storage.md",
        );
        assert_eq!(rfc.title, "RFC 0001: Storage");
        assert_eq!(rfc.status, "Accepted");
        assert_eq!(rfc.sections.len(), 3);
        assert_eq!(rfc.sections[1].anchor, "design");
        assert_eq!(rfc.sections[1].line, 3);
        assert!(rfc.sections[1].text.contains("# comment"));
        assert_eq!(rfc.sections[2].anchor, "design-1");
    }

    #[test]
    fn preserves_text_without_headings_or_metadata() {
        let rfc = parse("Use Redis for sessions.", "0002-storage.md");
        assert_eq!(rfc.title, "0002-storage.md");
        assert_eq!(rfc.status, "Unknown");
        assert_eq!(rfc.sections[0].text, "Use Redis for sessions.");
    }
}
