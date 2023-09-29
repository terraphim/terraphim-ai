extern crate pulldown_cmark;
use pulldown_cmark::{Options, Parser};
use std::collections::HashMap;

fn main() {
    let markdown_input = r#"
---
title: My Document
tags: [example, rust]
---

# Heading

This is a paragraph with a [[wikilink]].

Another paragraph with a [regular link](https://www.example.com).
"#;

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut frontmatter = HashMap::new();
    let parser = Parser::new_ext(markdown_input, options);

    for event in parser {
        if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading(level)) = event {
            if level == 1 {
                break;
            }
        } else if let pulldown_cmark::Event::Text(text) = event {
            if text.starts_with("---") {
                continue;
            }
            let mut parts = text.splitn(2, ':');
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                frontmatter.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    println!("{:#?}", frontmatter);
}