use terraphim_desktop_gpui::models::{ChipOperator, TermChip, TermChipSet};

#[test]
fn test_term_chip_set_add_remove() {
    let mut set = TermChipSet::new();

    set.add_chip(TermChip {
        value: "rust".to_string(),
        is_from_kg: true,
    });

    assert_eq!(set.chips.len(), 1);
    assert!(set.operator.is_none());

    set.add_chip(TermChip {
        value: "tokio".to_string(),
        is_from_kg: true,
    });

    assert_eq!(set.chips.len(), 2);
    assert_eq!(set.operator, Some(ChipOperator::And));

    set.remove_chip(0);
    assert_eq!(set.chips.len(), 1);
    assert!(set.operator.is_none());
}

#[test]
fn test_query_string_conversion() {
    let mut set = TermChipSet::new();
    set.add_chip(TermChip {
        value: "rust".to_string(),
        is_from_kg: true,
    });
    set.add_chip(TermChip {
        value: "async".to_string(),
        is_from_kg: true,
    });

    assert_eq!(set.to_query_string(), "rust AND async");

    set.operator = Some(ChipOperator::Or);
    assert_eq!(set.to_query_string(), "rust OR async");
}

#[test]
fn test_from_query_string() {
    let set = TermChipSet::from_query_string("rust AND tokio", |_| false);

    assert_eq!(set.chips.len(), 2);
    assert_eq!(set.chips[0].value, "rust");
    assert_eq!(set.chips[1].value, "tokio");
    assert_eq!(set.operator, Some(ChipOperator::And));
}

#[test]
fn test_clear_term_chips() {
    let mut set = TermChipSet::new();
    set.add_chip(TermChip {
        value: "rust".to_string(),
        is_from_kg: true,
    });
    set.add_chip(TermChip {
        value: "tokio".to_string(),
        is_from_kg: true,
    });

    assert!(!set.chips.is_empty());

    set.clear();
    assert!(set.chips.is_empty());
    assert!(set.operator.is_none());
}
