//! The documentation reference contract: every reference a doc comment makes
//! is mechanically resolved against the documents it names. A reference of
//! the form `docs/<file>.md` must name a file that exists; a reference of the
//! form `§N[.M]` must name a section of `docs/uvrr-durability-model.md` (the
//! spec, the default document for a bare `§N`) or of a combined-document
//! chapter the same comment names ("the reincarnation chapter §6"); a
//! decision identifier (`S2`, `W1`, `Q1`, `B1`, `G1`) must name an entry of
//! the decision record in `docs/architecture.md`; a rule identifier (`R14`)
//! must name a rule of the reconfiguration-rules chapter. `P`-identifiers are
//! ambiguous with the history invariants P1–P7 and are validated only when the
//! comment also names the decision record. Code spans and fenced code inside
//! doc comments are stripped before scanning: a reference is prose.
//!
//! The registry is parsed from the documents themselves at test time, and the
//! second test pins the registry's expected size and shape: a parser that
//! silently finds nothing cannot pass the contract vacuously.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// One combined-document chapter: its slug as references name it ("the
/// reincarnation chapter"), the file that carries it, the prefix of its
/// chapter heading, and its numbered sections.
struct Chapter {
    slug: &'static str,
    heading_prefix: &'static str,
    sections: BTreeSet<String>,
}

const COMBINED_FILES: [&str; 2] = ["docs/uvrr-protocols.md", "docs/uvrr-io-obligations.md"];

/// The chapters the combined documents must carry, by file, with the heading
/// prefixes the parser looks for. A chapter rename breaks the registry test.
const EXPECTED_CHAPTERS: [(&str, &str, &str); 7] = [
    (
        "docs/uvrr-io-obligations.md",
        "boot-gate",
        "## uVRR boot gate:",
    ),
    (
        "docs/uvrr-io-obligations.md",
        "termination",
        "## uVRR termination obligations:",
    ),
    (
        "docs/uvrr-protocols.md",
        "reconfiguration-rules",
        "## uVRR reconfiguration rules:",
    ),
    (
        "docs/uvrr-protocols.md",
        "reincarnation",
        "## uVRR reincarnation:",
    ),
    (
        "docs/uvrr-protocols.md",
        "rejoin",
        "## uVRR rejoin gossip and witnesses",
    ),
    (
        "docs/uvrr-protocols.md",
        "solver",
        "## Weighted reconfiguration solver",
    ),
    (
        "docs/uvrr-protocols.md",
        "NOMINATE",
        "## NOMINATE leader assignment",
    ),
];

struct Registry {
    /// Every `docs/*.md` path that exists, as the references name it.
    files: BTreeSet<String>,
    /// The spec's sections: `5`, `6.1`, ... from `docs/uvrr-durability-model.md`.
    spec_sections: BTreeSet<String>,
    /// The decision record's identifiers: `S2`, `W1`, ... from `docs/architecture.md`.
    decisions: BTreeSet<String>,
    /// The reconfiguration rules: `R1` .. `R15` from the rules chapter's tables.
    rules: BTreeSet<String>,
    /// The external works' citation identifiers: `[VR-2012]`, ... from
    /// `docs/references.md`.
    citations: BTreeSet<String>,
    /// The combined documents' chapters, by slug.
    chapters: BTreeMap<String, Chapter>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The leading section number of a heading line, if it carries one:
/// `## 5. Progress` or `### 6.1 Event-clock` yields `5` or `6.1`.
fn heading_number(line: &str) -> Option<String> {
    let text = line.trim_start_matches('#').trim_start();
    let mut number = String::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            number.push(c);
            chars.next();
        } else if c == '.' {
            // A dot continues the number only when a digit follows it.
            let mut lookahead = chars.clone();
            lookahead.next();
            if lookahead.next().is_some_and(|n| n.is_ascii_digit()) {
                number.push(c);
                chars.next();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if number.is_empty() {
        None
    } else {
        Some(number)
    }
}

/// The spec's sections live at every heading level: `## 5.` for the top
/// level, `### 6.1` and `#### 8.7.3` for the nested ones.
fn parse_spec_sections(text: &str) -> BTreeSet<String> {
    let mut sections = BTreeSet::new();
    for line in text.lines() {
        let level = line.chars().take_while(|c| *c == '#').count();
        if level >= 2
            && line[level..].starts_with(' ')
            && let Some(number) = heading_number(line)
        {
            sections.insert(number);
        }
    }
    sections
}

/// The citation identifiers of `docs/references.md`: the `## [ID]` headings.
fn parse_citations(text: &str) -> BTreeSet<String> {
    let mut citations = BTreeSet::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("## [")
            && let Some((id, _)) = rest.split_once(']')
            && !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
        {
            citations.insert(id.to_string());
        }
    }
    citations
}

fn parse_decisions(text: &str) -> BTreeSet<String> {
    let mut decisions = BTreeSet::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("#### ")
            && let Some((id, _)) = rest.split_once(',')
        {
            let id = id.trim();
            if id.len() >= 2
                && id.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && id[1..].chars().all(|c| c.is_ascii_digit())
            {
                decisions.insert(id.to_string());
            }
        }
    }
    decisions
}

/// The numbered sections beneath a chapter heading: every heading of level
/// three or deeper with a leading number, up to the next level-two heading.
fn parse_chapter_sections(text: &str, heading_prefix: &str) -> BTreeSet<String> {
    let mut sections = BTreeSet::new();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("## ") && !line.starts_with("### ") {
            inside = line.starts_with(heading_prefix);
            continue;
        }
        if inside
            && line.starts_with("###")
            && let Some(number) = heading_number(line)
        {
            sections.insert(number);
        }
    }
    sections
}

/// The rule identifiers of the rules chapter's tables: `| R14 | ... |` rows.
fn parse_rules(text: &str, chapter_prefix: &str) -> BTreeSet<String> {
    let mut rules = BTreeSet::new();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("## ") && !line.starts_with("### ") {
            inside = line.starts_with(chapter_prefix);
            continue;
        }
        if inside {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("| R") {
                let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !id.is_empty() && rest[id.len()..].trim_start().starts_with('|') {
                    rules.insert(format!("R{id}"));
                }
            }
        }
    }
    rules
}

fn build_registry(root: &Path) -> Registry {
    let read = |path: &str| {
        fs::read_to_string(root.join(path))
            .unwrap_or_else(|e| panic!("the registry reads {path}: {e}"))
    };
    let mut files = BTreeSet::new();
    for entry in fs::read_dir(root.join("docs")).expect("docs/ is a directory") {
        let name = entry.expect("a docs entry").file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".md") {
            files.insert(format!("docs/{name}"));
        }
    }
    let protocols = read("docs/uvrr-protocols.md");
    let mut chapters = BTreeMap::new();
    for (file, slug, prefix) in EXPECTED_CHAPTERS {
        let text = read(file);
        let sections = parse_chapter_sections(&text, prefix);
        chapters.insert(
            slug.to_string(),
            Chapter {
                slug,
                heading_prefix: prefix,
                sections,
            },
        );
    }
    Registry {
        files,
        spec_sections: parse_spec_sections(&read("docs/uvrr-durability-model.md")),
        decisions: parse_decisions(&read("docs/architecture.md")),
        rules: parse_rules(&protocols, "## uVRR reconfiguration rules:"),
        citations: parse_citations(&read("docs/references.md")),
        chapters,
    }
}

/// Every Rust source file under the scanned roots.
fn collect_sources(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("walk {}: {e}", dir.display())) {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    for top in ["src", "tests", "examples"] {
        walk(&root.join(top), &mut out);
    }
    out
}

/// A file's doc comments as contiguous blocks: `///` and `//!` lines, the
/// marker stripped, each block carrying the line it starts on, the raw
/// prose, and a cleaned prose with fenced code and inline code spans
/// removed. Document paths, section signs, and chapter names are scanned in
/// the raw prose (they are references even inside backticks, and real code
/// carries no section signs); the identifier tokens are scanned in the
/// cleaned prose, where a code example's `S2`-shaped name cannot
/// masquerade as a decision reference.
fn doc_comment_blocks(path: &Path) -> Vec<(usize, String, String)> {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut blocks: Vec<(usize, String, String)> = Vec::new();
    let mut current: Option<(usize, String, String)> = None;
    let mut in_fence = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let content = if let Some(rest) = trimmed.strip_prefix("///") {
            Some(rest.strip_prefix(' ').unwrap_or(rest))
        } else {
            trimmed
                .strip_prefix("//!")
                .map(|rest| rest.strip_prefix(' ').unwrap_or(rest))
        };
        match (content, &mut current) {
            (Some(content), Some((_, raw, cleaned))) => {
                push_line(raw, cleaned, content, &mut in_fence);
            }
            (Some(content), None) => {
                let mut raw = String::new();
                let mut cleaned = String::new();
                push_line(&mut raw, &mut cleaned, content, &mut in_fence);
                current = Some((index + 1, raw, cleaned));
            }
            (None, Some(_)) => {
                blocks.push(current.take().expect("an open block"));
                in_fence = false;
            }
            (None, None) => {}
        }
    }
    if let Some(block) = current.take() {
        blocks.push(block);
    }
    blocks
}

fn push_line(raw: &mut String, cleaned: &mut String, content: &str, in_fence: &mut bool) {
    if content.trim_start().starts_with("```") {
        *in_fence = !*in_fence;
        raw.push('\n');
        cleaned.push('\n');
        return;
    }
    if *in_fence {
        raw.push('\n');
        cleaned.push('\n');
        return;
    }
    raw.push_str(content);
    raw.push('\n');
    let mut stripped = String::with_capacity(content.len());
    let mut in_code = false;
    for c in content.chars() {
        if c == '`' {
            in_code = !in_code;
        } else if !in_code {
            stripped.push(c);
        }
    }
    cleaned.push_str(&stripped);
    cleaned.push('\n');
}

/// The tokens a cleaned block carries that the contract resolves: doc-file
/// references, section references, chapter mentions, decision identifiers,
/// and rule identifiers.
struct Tokens {
    files: Vec<String>,
    sections: Vec<String>,
    chapters: Vec<String>,
    decisions: Vec<String>,
    rules: Vec<String>,
    citations: Vec<String>,
}

fn scan(raw: &str, cleaned: &str, slugs: &[&str]) -> Tokens {
    let mut tokens = Tokens {
        files: Vec::new(),
        sections: Vec::new(),
        chapters: Vec::new(),
        decisions: Vec::new(),
        rules: Vec::new(),
        citations: Vec::new(),
    };
    // External citations: `[ID]` in the cleaned prose. An identifier
    // carries a digit or a hyphen (`[VR-2012]`, `[TURNER-RECONF]`), which
    // keeps bracketed operation names like `[DOUBLE]` out of the class.
    let cleaned_flat: String = cleaned
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    let mut rest = cleaned_flat.as_str();
    while let Some(at) = rest.find('[') {
        let after = &rest[at + 1..];
        let id: String = after
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        if id.len() >= 2
            && (id.chars().any(|c| c.is_ascii_digit()) || id.contains('-'))
            && after[id.len()..].starts_with(']')
        {
            tokens.citations.push(id);
        }
        rest = &after[1.min(after.len())..];
    }
    // A doc comment wraps at the line margin; the reference tokens do not
    // care, so the scans run over the flattened prose.
    let raw_flat: String = raw
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    // Doc-file references: `docs/<path>.md`.
    let mut rest = raw_flat.as_str();
    while let Some(at) = rest.find("docs/") {
        let after = &rest[at..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.')))
            .unwrap_or(after.len());
        let candidate = &after[..end];
        if let Some(path) = candidate.strip_suffix(".md") {
            tokens.files.push(format!("{path}.md"));
        } else if candidate.contains(".md") {
            // A path with trailing punctuation after `.md`: cut at it.
            if let Some(mark) = candidate.find(".md") {
                tokens.files.push(candidate[..mark + 3].to_string());
            }
        }
        rest = &after[end.max(1)..];
    }
    // Section references: `§` followed by a dotted number. A section sign
    // whose preceding prose ends with a citation's `]`, a digit, or `/`
    // names a section of the cited work or continues a multi-section cite
    // (`[VR-2012] §4.1/§4.2`), never a spec section, and is not validated
    // against the spec.
    for (at, _) in raw_flat.match_indices('§') {
        let before = raw_flat[..at].trim_end();
        let literature = before.ends_with(']')
            || before.ends_with('/')
            || before.chars().last().is_some_and(|c| c.is_ascii_digit());
        let after = &raw_flat[at + '§'.len_utf8()..];
        let number: String = after
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let number = number.trim_end_matches('.');
        if !number.is_empty() && !literature {
            tokens.sections.push(number.to_string());
        }
    }
    // Chapter mentions: `the <slug> chapter`.
    for slug in slugs {
        if raw_flat.contains(&format!("the {slug} chapter")) {
            tokens.chapters.push(slug.to_string());
        }
    }
    // Identifier tokens: alphanumeric runs of the cleaned prose.
    for word in cleaned.split(|c: char| !c.is_ascii_alphanumeric()) {
        if word.len() < 2 {
            continue;
        }
        let (head, tail) = word.split_at(1);
        if tail.chars().all(|c| c.is_ascii_digit()) && tail.len() <= 2 {
            match head {
                "S" | "W" | "Q" | "B" | "G" => tokens.decisions.push(word.to_string()),
                "R" => tokens.rules.push(word.to_string()),
                "P" if cleaned.contains("architecture.md") || cleaned.contains("Decision") => {
                    tokens.decisions.push(word.to_string());
                }
                _ => {}
            }
        }
    }
    tokens
}

/// Every reference in every doc comment of every scanned source file
/// resolves: files exist, sections name the spec or a named chapter,
/// combined-document references name a chapter, and the decision and rule
/// identifiers name entries of their records.
#[test]
fn every_doc_comment_reference_resolves() {
    let root = repo_root();
    let registry = build_registry(&root);
    let slugs: Vec<&str> = registry.chapters.values().map(|c| c.slug).collect();
    let mut violations: Vec<String> = Vec::new();
    for path in collect_sources(&root) {
        let relative = path
            .strip_prefix(&root)
            .expect("sources live under the root")
            .to_path_buf();
        for (line, raw, cleaned) in doc_comment_blocks(&path) {
            let tokens = scan(&raw, &cleaned, &slugs);
            for file in &tokens.files {
                if !registry.files.contains(file) {
                    violations.push(format!(
                        "{}:{line}: the document `{file}` does not exist",
                        relative.display()
                    ));
                    continue;
                }
                if COMBINED_FILES.contains(&file.as_str()) {
                    let names_a_chapter = tokens.chapters.iter().any(|slug| {
                        registry.chapters[slug].heading_prefix.starts_with("## ")
                            && chapter_file(slug, &registry) == *file
                    });
                    if !names_a_chapter {
                        violations.push(format!(
                            "{}:{line}: `{file}` is a combined document: name the chapter",
                            relative.display()
                        ));
                    }
                }
            }
            for section in &tokens.sections {
                let in_spec = registry.spec_sections.contains(section);
                let in_chapter = tokens
                    .chapters
                    .iter()
                    .any(|slug| registry.chapters[slug].sections.contains(section));
                if !in_spec && !in_chapter {
                    violations.push(format!(
                        "{}:{line}: §{section} names no section of the spec{}",
                        relative.display(),
                        if tokens.chapters.is_empty() {
                            " (and no chapter is named)".to_string()
                        } else {
                            format!(
                                " or of the named chapter(s): {}",
                                tokens.chapters.join(", ")
                            )
                        }
                    ));
                }
            }
            for decision in &tokens.decisions {
                if !registry.decisions.contains(decision) {
                    violations.push(format!(
                        "{}:{line}: {decision} names no entry of the decision record",
                        relative.display()
                    ));
                }
            }
            for rule in &tokens.rules {
                if !registry.rules.contains(rule) {
                    violations.push(format!(
                        "{}:{line}: {rule} names no rule of the reconfiguration-rules chapter",
                        relative.display()
                    ));
                }
            }
            for citation in &tokens.citations {
                if !registry.citations.contains(citation) {
                    violations.push(format!(
                        "{}:{line}: [{citation}] names no entry of `docs/references.md`",
                        relative.display()
                    ));
                } else if !tokens.files.iter().any(|f| f == "docs/references.md") {
                    violations.push(format!(
                        "{}:{line}: cites [{citation}] without naming `docs/references.md`",
                        relative.display()
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "dangling documentation references:\n{}",
        violations.join("\n")
    );
}

fn chapter_file(slug: &str, registry: &Registry) -> String {
    for (file, chapter_slug, _) in EXPECTED_CHAPTERS {
        if chapter_slug == slug {
            return file.to_string();
        }
    }
    let _ = registry;
    unreachable!("every slug comes from EXPECTED_CHAPTERS");
}

/// The registry is parsed from the documents, and a parser that finds
/// nothing would pass the contract vacuously: pin the registry's shape so a
/// documents reformat that defeats the parser fails here, loudly.
///
/// The rules of the road for this pin: the pin does NOT freeze the
/// documentation. The documents are living documents that travel with the
/// code, and they will grow, split, merge, and renumber. What the pin
/// forbids is DRIFT — a code comment naming an anchor the documents no
/// longer carry, or a documents restructure that silently strands the
/// code's references. When the documentation genuinely moves, the honest
/// moves are: update the reference, update the pin, or — when the target
/// section is not written yet — add the section to the document with a
/// `TODO` as its content and reference that. A questioned drift is the
/// gate working; a silent drift is the failure it exists to catch.
#[test]
fn the_registry_covers_the_expected_anchors() {
    let registry = build_registry(&repo_root());
    assert!(
        registry.spec_sections.len() >= 20,
        "the spec carries its sections: found {}",
        registry.spec_sections.len()
    );
    for expected in ["1.3", "5", "6.1", "6.2", "8.7.3", "13.4", "14.2"] {
        assert!(
            registry.spec_sections.contains(expected),
            "the spec carries §{expected}"
        );
    }
    for expected in ["W1", "S2", "S3", "S4", "Q1", "Q2", "B1", "G1", "P1"] {
        assert!(
            registry.decisions.contains(expected),
            "the decision record carries {expected}"
        );
    }
    assert_eq!(
        registry.rules.len(),
        15,
        "the reconfiguration rules are R1..R15: found {:?}",
        registry.rules
    );
    for expected in [
        "VR-2012",
        "PMS-2001",
        "DISC-2017",
        "OSDI-2014",
        "SOSP-2013",
        "LAMPSON-1979",
    ] {
        assert!(
            registry.citations.contains(expected),
            "the references carry [{expected}]"
        );
    }
    for (file, slug, prefix) in EXPECTED_CHAPTERS {
        let chapter = registry
            .chapters
            .get(slug)
            .unwrap_or_else(|| panic!("the {slug} chapter exists in {file}"));
        assert_eq!(chapter.heading_prefix, prefix);
        assert!(
            registry.files.contains(file),
            "the combined document {file} exists"
        );
    }
    for (slug, count) in [
        ("boot-gate", 8),
        ("termination", 4),
        ("reconfiguration-rules", 11),
        ("reincarnation", 10),
        ("rejoin", 6),
    ] {
        let chapter = &registry.chapters[slug];
        assert_eq!(
            chapter.sections.len(),
            count,
            "the {slug} chapter carries its {count} sections: found {:?}",
            chapter.sections
        );
    }
}
