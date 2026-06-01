use fff_search::types::FileItem;
use fff_search::{FilePicker, PaginationArgs, SearchResult};

use crate::kg_scorer::KgPathScorer;

/// Minimum candidate limit used when Terraphim ranking re-ranks before final paging.
pub const DEFAULT_RANKING_CANDIDATE_LIMIT: usize = 1000;

/// Candidate metadata available immediately after fff-search candidate retrieval.
pub struct FileRankCandidate<'a> {
    pub relative_path: &'a str,
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
}

/// Scores file/search candidates for Terraphim-specific ranking.
pub trait FileRanker {
    fn score_candidate(&self, candidate: &FileRankCandidate<'_>) -> i32;
}

pub struct KgFileRanker<'a> {
    scorer: &'a KgPathScorer,
}

impl<'a> KgFileRanker<'a> {
    pub fn new(scorer: &'a KgPathScorer) -> Self {
        Self { scorer }
    }
}

impl FileRanker for KgFileRanker<'_> {
    fn score_candidate(&self, candidate: &FileRankCandidate<'_>) -> i32 {
        self.scorer.score_path(candidate.relative_path)
    }
}

/// Return the fff-search pagination to use for Terraphim-ranked candidate collection.
pub fn widened_pagination(final_offset: usize, final_limit: usize) -> PaginationArgs {
    PaginationArgs {
        offset: 0,
        limit: DEFAULT_RANKING_CANDIDATE_LIMIT.max(final_offset.saturating_add(final_limit)),
    }
}

/// Apply Terraphim ranking boost to fuzzy results, sort, then apply final pagination.
pub fn rank_fuzzy_results<'a>(
    picker: &FilePicker,
    results: SearchResult<'a>,
    ranker: &dyn FileRanker,
    final_offset: usize,
    final_limit: usize,
) -> SearchResult<'a> {
    let mut ranked: Vec<_> = results
        .items
        .into_iter()
        .zip(results.scores)
        .enumerate()
        .map(|(index, (file, mut score))| {
            let relative_path = file.relative_path(picker);
            let candidate = FileRankCandidate {
                relative_path: &relative_path,
                title: None,
                body: None,
            };
            score.total += ranker.score_candidate(&candidate);
            (index, file, score)
        })
        .collect();

    ranked.sort_by(|a, b| b.2.total.cmp(&a.2.total).then_with(|| a.0.cmp(&b.0)));

    let page = ranked.into_iter().skip(final_offset).take(final_limit);
    let (items, scores): (Vec<_>, Vec<_>) = page.map(|(_, file, score)| (file, score)).unzip();

    SearchResult {
        items,
        scores,
        total_matched: results.total_matched,
        total_files: results.total_files,
        location: results.location,
    }
}

/// Sort file refs by Terraphim ranking before grep so match file indices remain valid.
pub fn sort_files_by_rank<'a>(
    picker: &FilePicker,
    files: Vec<&'a FileItem>,
    ranker: &dyn FileRanker,
) -> Vec<&'a FileItem> {
    let mut ranked: Vec<_> = files
        .into_iter()
        .enumerate()
        .map(|(index, file)| {
            let relative_path = file.relative_path(picker);
            let candidate = FileRankCandidate {
                relative_path: &relative_path,
                title: None,
                body: None,
            };
            (index, ranker.score_candidate(&candidate), file)
        })
        .collect();

    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.into_iter().map(|(_, _, file)| file).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fff_search::{FFFMode, FilePickerOptions, FuzzySearchOptions, QueryParser};

    struct PathRanker;

    impl FileRanker for PathRanker {
        fn score_candidate(&self, candidate: &FileRankCandidate<'_>) -> i32 {
            if candidate.relative_path.contains("priority") {
                1000
            } else {
                0
            }
        }
    }

    fn picker_with_files() -> (tempfile::TempDir, FilePicker) {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("neutral.rs"), "fn neutral() {}").unwrap();
        std::fs::write(dir.path().join("priority.rs"), "fn priority() {}").unwrap();

        let mut picker = FilePicker::new(FilePickerOptions {
            base_path: dir.path().to_string_lossy().to_string(),
            mode: FFFMode::Ai,
            watch: false,
            cache_budget: None,
            ..FilePickerOptions::default()
        })
        .unwrap();
        picker.collect_files().unwrap();
        (dir, picker)
    }

    #[test]
    fn widened_pagination_covers_final_page() {
        let pagination = widened_pagination(50, 25);
        assert_eq!(pagination.offset, 0);
        assert!(pagination.limit >= 75);
    }

    #[test]
    fn rank_fuzzy_results_applies_rank_before_final_page() {
        let (_dir, picker) = picker_with_files();
        let parser = QueryParser::default();
        let query = parser.parse("rs");
        let results = picker.fuzzy_search(
            &query,
            None,
            FuzzySearchOptions {
                pagination: PaginationArgs {
                    offset: 0,
                    limit: 10,
                },
                ..FuzzySearchOptions::default()
            },
        );

        let ranked = rank_fuzzy_results(&picker, results, &PathRanker, 0, 1);

        assert_eq!(ranked.items.len(), 1);
        assert_eq!(ranked.items[0].relative_path(&picker), "priority.rs");
    }

    #[test]
    fn sort_files_by_rank_preserves_file_refs_in_rank_order() {
        let (_dir, picker) = picker_with_files();
        let files: Vec<_> = picker.get_files().iter().collect();

        let sorted = sort_files_by_rank(&picker, files, &PathRanker);

        assert_eq!(sorted[0].relative_path(&picker), "priority.rs");
        assert_eq!(sorted.len(), 2);
    }
}
