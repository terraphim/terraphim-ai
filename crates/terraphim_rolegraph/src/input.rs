/// Incrementally growing system-operator test document (1 section).
pub const TEST1: &str = include_str!("../data/system_operator_cc/test1.md");
/// Incrementally growing system-operator test document (2 sections).
pub const TEST12: &str = include_str!("../data/system_operator_cc/test12.md");
/// Incrementally growing system-operator test document (3 sections).
pub const TEST123: &str = include_str!("../data/system_operator_cc/test123.md");
/// Incrementally growing system-operator test document (4 sections).
pub const TEST1234: &str = include_str!("../data/system_operator_cc/test1234.md");
/// Incrementally growing system-operator test document (5 sections).
pub const TEST12345: &str = include_str!("../data/system_operator_cc/test12345.md");
/// Ordered slice of all test corpus documents for bulk indexing tests.
pub const TEST_CORPUS: &[&str] = &[TEST1, TEST12, TEST123, TEST1234, TEST12345];
