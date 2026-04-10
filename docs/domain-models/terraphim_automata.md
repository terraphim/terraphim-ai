# terraphim_automata - Text Matching and Autocomplete Engine

## Overview

`terraphim_automata` provides high-performance text processing using Aho-Corasick automata and finite state transducers (FST). It powers Terraphim's autocomplete and knowledge graph linking features with support for fuzzy matching, link generation, and paragraph extraction.

## Domain Model

### Core Concepts

#### AutomataPath
Path to a thesaurus/automata file, either local or remote.

```rust
pub enum AutomataPath {
    Local(PathBuf),
    Remote(String),
}
```

**Key Responsibilities:**
- Abstract file location (local or remote)
- Support HTTP/HTTPS URLs
- Provide consistent interface

#### Thesaurus
Mapping from normalised terms to concepts with metadata.

```rust
pub struct Thesaurus {
    pub name: String,
    pub terms: AHashMap<NormalisedTermValue, NormalisedTerm>,
}
```

**Key Responsibilities:**
- Store term-to-concept mappings
- Support synonym lookups
- Provide iteration interface

#### LinkType
Format for link generation in text replacement.

```rust
pub enum LinkType {
    MarkdownLinks,
    HTMLLinks,
    WikiLinks,
}
```

**Key Responsibilities:**
- Define output format for matches
- Support multiple link types
- Enable custom formatting

### Autocomplete

#### AutocompleteIndex
FST-based prefix search index with metadata.

```rust
pub struct AutocompleteIndex {
    pub fst: fst::Map<Vec<u8>>,
    pub metadata: AHashMap<String, AutocompleteMetadata>,
}
```

**Key Responsibilities:**
- Fast prefix-based search
- Store term metadata
- Support fuzzy matching

#### AutocompleteMetadata
Metadata associated with autocomplete terms.

```rust
pub struct AutocompleteMetadata {
    pub id: u64,
    pub url: Option<String>,
    pub display_value: Option<String>,
}
```

**Key Responsibilities:**
- Store term identifiers
- Link to external resources
- Preserve display values

#### AutocompleteResult
Result from autocomplete search.

```rust
pub struct AutocompleteResult {
    pub term: String,
    pub score: f64,
    pub metadata: AutocompleteMetadata,
}
```

**Key Responsibilities:**
- Return matched term
- Provide relevance score
- Include metadata

### Text Matching

#### Matched
Single text match with location information.

```rust
pub struct Matched {
    pub matched: String,
    pub start: usize,
    pub end: usize,
    pub metadata: NormalisedTerm,
}
```

**Key Responsibilities:**
- Store matched text
- Track position in source
- Provide term metadata

#### MatchedIterator
Iterator over all matches in text.

```rust
pub struct MatchedIterator<'a> {
    // Internal state for iteration
}
```

**Key Responsibilities:**
- Iterate over matches
- Preserve order
- Efficient traversal

## Data Models

### Thesaurus Builder

#### ThesaurusBuilder
Builder pattern for constructing thesaurus from sources.

```rust
pub trait ThesaurusBuilder: Send + Sync {
    async fn build(
        &self,
        role: String,
        path: String
    ) -> Result<Thesaurus>;
}
```

**Implementations:**
- `Logseq`: Build from Logseq markdown files
- `JsonThesaurusBuilder`: Build from JSON files

#### Logseq
Logseq-specific thesaurus builder.

```rust
pub struct Logseq {
    pub path: String,
}
```

**Key Responsibilities:**
- Parse Logseq markdown files
- Extract term definitions
- Build thesaurus structure

## Implementation Patterns

### Thesaurus Loading

#### Local File Loading
```rust
pub fn load_thesaurus(automata_path: &AutomataPath) -> Result<Thesaurus> {
    let contents = match automata_path {
        AutomataPath::Local(path) => fs::read_to_string(path)?,
        AutomataPath::Remote(_) => {
            return Err(TerraphimAutomataError::InvalidThesaurus(
                "Remote loading is not supported. Enable 'remote-loading' feature.".to_string(),
            ));
        }
    };

    parse_thesaurus_json(&contents)
}
```

**Pattern:**
- Match on path type
- Read file contents
- Parse JSON structure
- Handle errors gracefully

#### Remote Loading (feature: "remote-loading")
```rust
pub async fn load_thesaurus(automata_path: &AutomataPath) -> Result<Thesaurus> {
    async fn read_url(url: String) -> Result<String> {
        log::debug!("Reading thesaurus from remote: {url}");
        let response = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Terraphim-Automata/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
            .get(url.clone())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                TerraphimAutomataError::InvalidThesaurus(format!(
                    "Failed to fetch thesaurus from remote {url}. Error: {e:#?}",
                ))
            })?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await;

        match body {
            Ok(text) => Ok(text),
            Err(e) => {
                let error_info = format!(
                    "Failed to read thesaurus from remote {url}. Status: {status}. Headers: {headers:#?}. Error: {e:#?}",
                );
                Err(TerraphimAutomataError::InvalidThesaurus(error_info))
            }
        }
    }

    let contents = match automata_path {
        AutomataPath::Local(path) => {
            if !std::path::Path::new(path).exists() {
                return Err(TerraphimAutomataError::InvalidThesaurus(format!(
                    "Thesaurus file not found: {}",
                    path.display()
                )));
            }
            fs::read_to_string(path)?
        }
        AutomataPath::Remote(url) => read_url(url.clone()).await?,
    };

    parse_thesaurus_json(&contents)
}
```

**Pattern:**
- Async HTTP request with timeout
- Error handling with context
- Status and header logging
- Graceful degradation

### Autocomplete Building

#### Index Construction
```rust
pub fn build_autocomplete_index(
    thesaurus: &Thesaurus,
    config: Option<AutocompleteConfig>
) -> Result<AutocompleteIndex> {
    let mut builder = fst::MapBuilder::new();

    for (key, term) in thesaurus {
        let metadata = AutocompleteMetadata {
            id: term.id,
            url: term.url.clone(),
            display_value: term.display_value.clone(),
        };

        let metadata_json = serde_json::to_string(&metadata)?;
        let mut entry = Vec::new();
        entry.extend_from_slice(key.as_str().as_bytes());
        entry.extend_from_slice(b"\0");
        entry.extend_from_slice(metadata_json.as_bytes());

        builder.insert(entry)?;
    }

    let fst_bytes = builder.into_inner()?;
    let fst = fst::Map::new(fst_bytes)?;

    let mut metadata = AHashMap::new();
    for (key, term) in thesaurus {
        let meta = AutocompleteMetadata {
            id: term.id,
            url: term.url.clone(),
            display_value: term.display_value.clone(),
        };
        metadata.insert(key.as_str().to_string(), meta);
    }

    Ok(AutocompleteIndex { fst, metadata })
}
```

**Pattern:**
- Create FST builder
- Insert term + metadata pairs
- Use null byte separator
- Build separate metadata map

#### Prefix Search
```rust
pub fn autocomplete_search(
    index: &AutocompleteIndex,
    prefix: &str,
    limit: Option<usize>
) -> Vec<AutocompleteResult> {
    let mut results = Vec::new();
    let mut op = index.fst.stream();

    op.seek(prefix.as_bytes()).ok();

    while let Some((key, _value)) = op.next() {
        if !key.starts_with(prefix.as_bytes()) {
            break;
        }

        let parts: Vec<&[u8]> = key.splitn(0).collect();
        if parts.len() < 2 {
            continue;
        }

        let term_str = String::from_utf8_lossy(parts[0]);
        let metadata_json = String::from_utf8_lossy(parts[1]);

        if let Ok(metadata) = serde_json::from_str::<AutocompleteMetadata>(&metadata_json) {
            results.push(AutocompleteResult {
                term: term_str,
                score: 1.0,
                metadata,
            });

            if let Some(limit) = limit {
                if results.len() >= limit {
                    break;
                }
            }
        }
    }

    results
}
```

**Pattern:**
- Seek to prefix in FST
- Stream matching entries
- Parse metadata from value
- Apply result limit
- Handle UTF-8 loss gracefully

### Fuzzy Matching

#### Jaro-Winkler Similarity
```rust
pub fn fuzzy_autocomplete_search_jaro_winkler(
    index: &AutocompleteIndex,
    query: &str,
    threshold: f64,
    limit: Option<usize>
) -> Result<Vec<AutocompleteResult>> {
    let mut results = Vec::new();

    for (term_str, metadata) in &index.metadata {
        let distance = jaro_winkler_similarity(query, term_str);

        if distance >= threshold {
            results.push(AutocompleteResult {
                term: term_str.clone(),
                score: distance,
                metadata: metadata.clone(),
            });

            if let Some(limit) = limit {
                if results.len() >= limit {
                    break;
                }
            }
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(results)
}

fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
    // Jaro-Winkler distance calculation
    // Implementation omitted for brevity
}
```

**Pattern:**
- Iterate over all terms
- Compute Jaro-Winkler similarity
- Filter by threshold
- Sort by score
- Apply limit

#### Levenshtein Distance
```rust
pub fn fuzzy_autocomplete_search_levenshtein(
    index: &AutocompleteIndex,
    query: &str,
    threshold: f64,
    limit: Option<usize>
) -> Result<Vec<AutocompleteResult>> {
    let mut results = Vec::new();

    for (term_str, metadata) in &index.metadata {
        let distance = levenshtein_distance(query, term_str);
        let max_len = query.len().max(term_str.len());
        let similarity = 1.0 - (distance as f64 / max_len as f64);

        if similarity >= threshold {
            results.push(AutocompleteResult {
                term: term_str.clone(),
                score: similarity,
                metadata: metadata.clone(),
            });

            if let Some(limit) = limit {
                if results.len() >= limit {
                    break;
                }
            }
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(results)
}
```

**Pattern:**
- Compute Levenshtein distance
- Normalise by maximum length
- Filter and sort by similarity
- Apply limit

### Text Matching and Replacement

#### Finding Matches
```rust
pub fn find_matches(
    text: &str,
    thesaurus: &Thesaurus
) -> Result<Vec<Matched>> {
    let (ac, values, ac_reverse_nterm) = build_aho_corasick(thesaurus)?;

    let mut matches = Vec::new();
    for mat in ac.find_iter(text) {
        let term_id = values[mat.pattern()];
        let nterm = ac_reverse_nterm.get(&term_id)
            .ok_or_else(|| {
                TerraphimAutomataError::Dict(format!(
                    "No reverse lookup for term ID {}",
                    term_id
                ))
            })?;

        matches.push(Matched {
            matched: mat.pattern().to_string(),
            start: mat.start(),
            end: mat.end(),
            metadata: NormalisedTerm {
                id: term_id,
                value: nterm.clone(),
                display_value: None,
                url: None,
            },
        });
    }

    Ok(matches)
}
```

**Pattern:**
- Build Aho-Corasick automata
- Iterate over matches
- Reverse lookup term metadata
- Return structured results

#### Replacing Matches
```rust
pub fn replace_matches(
    content: &str,
    thesaurus: &Thesaurus,
    link_type: LinkType
) -> Result<Vec<u8>> {
    let matches = find_matches(content, thesaurus)?;

    if matches.is_empty() {
        return Ok(content.as_bytes().to_vec());
    }

    let mut result = Vec::new();
    let mut last_end = 0;

    for mat in &matches {
        // Append text before match
        result.extend_from_slice(content[last_end..mat.start].as_bytes());

        // Generate link based on type
        let link = generate_link(mat, link_type);
        result.extend_from_slice(link.as_bytes());

        last_end = mat.end;
    }

    // Append remaining text
    result.extend_from_slice(content[last_end..].as_bytes());

    Ok(result)
}

fn generate_link(mat: &Matched, link_type: &LinkType) -> String {
    let display = mat.metadata.display();
    let url = mat.metadata.url.as_deref().unwrap_or("");

    match link_type {
        LinkType::MarkdownLinks => {
            if url.is_empty() {
                format!("[{}]", display)
            } else {
                format!("[{}]({})", display, url)
            }
        }
        LinkType::HTMLLinks => {
            if url.is_empty() {
                format!("<span>{}</span>", display)
            } else {
                format!("<a href=\"{url}\">{display}</a>", url, display)
            }
        }
        LinkType::WikiLinks => {
            format!("[[{}]]", display)
        }
    }
}
```

**Pattern:**
- Find all matches first
- Iterate in order
- Replace with generated links
- Preserve unmatched text
- Handle empty URLs gracefully

### Paragraph Extraction

#### Extracting Context
```rust
pub fn extract_paragraphs_from_automata(
    content: &str,
    thesaurus: &Thesaurus,
    context_lines: usize
) -> Result<Vec<String>> {
    let matches = find_matches(content, thesaurus)?;

    let mut paragraphs = Vec::new();
    for mat in &matches {
        let paragraph = extract_paragraph_around_match(content, mat, context_lines);
        paragraphs.push(paragraph);
    }

    Ok(paragraphs)
}

fn extract_paragraph_around_match(
    content: &str,
    mat: &Matched,
    context_lines: usize
) -> String {
    let lines: Vec<&str> = content.lines().collect();

    let match_line_idx = content[..mat.start()]
        .matches(char::is_whitespace)
        .count();

    let start_idx = match_line_idx.saturating_sub(0);
    let end_idx = (match_line_idx + context_lines + 1).min(lines.len());

    lines[start_idx..end_idx].join("\n")
}
```

**Pattern:**
- Find all matches
- Extract surrounding context
- Use line-based extraction
- Apply context window

## Error Handling

### Error Types

```rust
#[derive(thiserror::Error, Debug)]
pub enum TerraphimAutomataError {
    #[error("Invalid thesaurus: {0}")]
    InvalidThesaurus(String),

    #[error("Serde deserialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Dict error: {0}")]
    Dict(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Aho-Corasick build error: {0}")]
    AhoCorasick(#[from] aho_corasick::BuildError),

    #[error("FST error: {0}")]
    Fst(#[from] fst::Error),
}
```

**Categories:**
- **Validation**: Invalid thesaurus format
- **Serialisation**: JSON parsing errors
- **I/O**: File system errors
- **Automata**: Aho-Corasick and FST errors

## Performance Optimisations

### FST Indexing

#### Memory Layout
```rust
let mut entry = Vec::new();
entry.extend_from_slice(key.as_str().as_bytes());
entry.extend_from_slice(b"\0");
entry.extend_from_slice(metadata_json.as_bytes());
```

**Optimisation:**
- Store term and metadata together
- Use null byte as separator
- Fast prefix matching via FST
- Compact storage format

### Caching

#### Metadata Storage
```rust
pub struct AutocompleteIndex {
    pub fst: fst::Map<Vec<u8>>,
    pub metadata: AHashMap<String, AutocompleteMetadata>,
}
```

**Optimisation:**
- FST for fast prefix search
- Separate map for metadata
- Fast lookup by term
- Avoid JSON parsing during search

### Lazy Evaluation

#### Streaming Iterator
```rust
pub fn autocomplete_search(
    index: &AutocompleteIndex,
    prefix: &str,
    limit: Option<usize>
) -> Vec<AutocompleteResult> {
    let mut results = Vec::new();
    let mut op = index.fst.stream();

    op.seek(prefix.as_bytes()).ok();

    while let Some((key, _value)) = op.next() {
        // Process matches as they arrive
        // Apply limit early to exit loop
    }

    results
}
```

**Optimisation:**
- Stream results from FST
- Process incrementally
- Early exit on limit
- Avoid full iteration

## Testing Patterns

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_thesaurus_from_json() {
        let json_str = r#"{
          "name": "Engineering",
          "data": {
            "rust": {
              "id": 1,
              "nterm": "rust programming",
              "url": "https://rust-lang.org"
            }
          }
        }"#;

        let thesaurus = load_thesaurus_from_json(json_str).unwrap();
        assert_eq!(thesaurus.len(), 1);
    }

    #[test]
    fn test_autocomplete_search() {
        let mut thesaurus = Thesaurus::new("test".to_string());
        thesaurus.insert(
            NormalisedTermValue::from("rust"),
            NormalisedTerm::new(1, NormalisedTermValue::from("rust"))
        );

        let index = build_autocomplete_index(&thesaurus, None).unwrap();
        let results = autocomplete_search(&index, "ru", None);

        assert!(!results.is_empty());
        assert!(results[0].term.starts_with("ru"));
    }

    #[test]
    fn test_fuzzy_search() {
        let mut thesaurus = Thesaurus::new("test".to_string());
        thesaurus.insert(
            NormalisedTermValue::from("programming"),
            NormalisedTerm::new(1, NormalisedTermValue::from("programming"))
        );

        let index = build_autocomplete_index(&thesaurus, None).unwrap();
        let results = fuzzy_autocomplete_search_jaro_winkler(
            &index,
            "programin",
            0.8,
            None
        ).unwrap();

        assert!(!results.is_empty());
        assert!(results[0].score >= 0.8);
    }

    #[test]
    fn test_replace_matches() {
        let mut thesaurus = Thesaurus::new("test".to_string());
        thesaurus.insert(
            NormalisedTermValue::from("rust"),
            NormalisedTerm::with_auto_id(
                NormalisedTermValue::from("rust")
            ).with_url("https://rust-lang.org".to_string())
        );

        let text = "I love rust!";
        let replaced = replace_matches(
            text,
            &thesaurus,
            LinkType::MarkdownLinks
        ).unwrap();

        let result_str = String::from_utf8(replaced).unwrap();
        assert!(result_str.contains("[rust](https://rust-lang.org)"));
    }
}
```

## Best Practices

### Thesaurus Design

- Use unique IDs consistently
- Normalise terms case-insensitively
- Provide display values for output
- Support external resource links

### Autocomplete

- Use FST for prefix matching
- Separate metadata storage
- Support fuzzy matching
- Apply sensible limits

### Text Matching

- Use Aho-Corasick for efficiency
- Handle overlapping matches correctly
- Preserve original text structure
- Support multiple link formats

### Error Handling

- Provide context in errors
- Categorise error types
- Use `thiserror` for consistency
- Handle recoverable errors gracefully

## Future Enhancements

### Planned Features

#### Stemming Support
```rust
pub fn autocomplete_search_stemmed(
    index: &AutocompleteIndex,
    prefix: &str,
    stemmer: &dyn Stemmer
) -> Vec<AutocompleteResult> {
    // Apply stemming before search
}
```

#### Phonetic Matching
```rust
pub fn fuzzy_autocomplete_search_phonetic(
    index: &AutocompleteIndex,
    query: &str,
    limit: Option<usize>
) -> Vec<AutocompleteResult> {
    // Use phonetic algorithms
}
```

#### Context-Aware Matching
```rust
pub fn find_matches_with_context(
    content: &str,
    thesaurus: &Thesaurus,
    window_size: usize
) -> Result<Vec<MatchedWithContext>> {
    // Provide surrounding context in results
}
```

## References

- [Aho-Corasick algorithm](https://github.com/BurntSushi/aho-corasick)
- [FST documentation](https://github.com/BurntSushi/fst-rs)
- [Jaro-Winkler distance](https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance)
- [Levenshtein distance](https://en.wikipedia.org/wiki/Levenshtein_distance)
- [ThisError for error handling](https://docs.rs/thiserror/)
