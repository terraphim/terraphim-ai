extern crate pulldown_cmark;
use pulldown_cmark::{html, Options, Parser, Tag};

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

    let parser = Parser::new_ext(markdown_input, options).map(|event| match event {
        pulldown_cmark::Event::Text(text) => {
            if text.starts_with("[[") && text.ends_with("]]") {
                let link_text = text.trim_matches(|c| c == '[' || c == ']');
                pulldown_cmark::Event::Start(Tag::Link {
                    link_type: pulldown_cmark::LinkType::Shortcut,
                    dest_url: link_text.to_string().into(),
                    title: link_text.to_string().into(),
                    id: "".into(),
                })
            } else {
                pulldown_cmark::Event::Text(text)
            }
        }
        _ => event,
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    println!("{}", html_output);
}
