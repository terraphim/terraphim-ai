use terraphim_automata::matcher::extract_paragraphs_from_automata;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

fn make_thesaurus(terms: &[&str], norm: &str) -> Thesaurus {
    let mut t = Thesaurus::new("test".to_string());
    let normalized = NormalizedTerm::new(1, NormalizedTermValue::from(norm));
    for term in terms {
        t.insert(NormalizedTermValue::from(*term), normalized.clone());
    }
    t
}

#[test]
fn finds_paragraph_from_term_start_including_term() {
    let thesaurus = make_thesaurus(&["lorem"], "lorem");
    let text =
        "Intro\n\nlorem ipsum dolor sit amet,\nconsectetur adipiscing elit.\n\nNext paragraph.";
    let res = extract_paragraphs_from_automata(text, thesaurus, true).unwrap();
    assert_eq!(res.len(), 1);
    let (_m, para) = &res[0];
    assert!(para.starts_with("lorem ipsum"));
    assert!(para.contains("consectetur"));
    assert!(!para.contains("Next paragraph"));
}

#[test]
fn finds_paragraph_from_term_start_excluding_term() {
    let thesaurus = make_thesaurus(&["lorem"], "lorem");
    let text = "Intro\n\nlorem ipsum dolor sit amet\n\nTail";
    let res = extract_paragraphs_from_automata(text, thesaurus, false).unwrap();
    assert_eq!(res.len(), 1);
    let (_m, para) = &res[0];
    assert!(para.starts_with(" ipsum")); // starts right after the term
}

#[test]
fn multiple_matches_same_paragraph_return_each_slice() {
    let thesaurus = make_thesaurus(&["alpha", "beta"], "norm");
    let text = "alpha ... middle ... beta\nline 2\n\nTail";
    let res = extract_paragraphs_from_automata(text, thesaurus, true).unwrap();
    // Both terms present in same paragraph -> two slices
    assert_eq!(res.len(), 2);
    // Both slices end before Tail
    assert!(res.iter().all(|(_, p)| !p.contains("Tail")));
}

#[test]
fn finds_end_of_text_when_no_blank_line() {
    let thesaurus = make_thesaurus(&["end"], "end");
    let text = "Prefix\n\nend of file with no blank line";
    let res = extract_paragraphs_from_automata(text, thesaurus, true).unwrap();
    assert_eq!(res.len(), 1);
    assert!(res[0].1.ends_with("blank line"));
}

#[test]
fn windows_crlf_paragraph_split() {
    let thesaurus = make_thesaurus(&["term"], "term");
    let text = "p1\r\n\r\nterm starts here and continues\r\nline2\r\n\r\nnext";
    let res = extract_paragraphs_from_automata(text, thesaurus, true).unwrap();
    assert_eq!(res.len(), 1);
    assert!(res[0].1.contains("line2"));
    assert!(!res[0].1.contains("next"));
}

#[test]
fn case_insensitive_matching() {
    let thesaurus = make_thesaurus(&["FoO"], "foo");
    let text = "\nfoo para\n\nEND";
    let res = extract_paragraphs_from_automata(text, thesaurus, true).unwrap();
    assert_eq!(res.len(), 1);
}

#[test]
fn no_matches_returns_empty() {
    let thesaurus = make_thesaurus(&["x"], "x");
    let text = "no paragraphs with match";
    let res = extract_paragraphs_from_automata(text, thesaurus, true).unwrap();
    assert!(res.is_empty());
}
