use terraphim_pipeline::IndexedDocument;
use terraphim_types::{merge_and_serialize, Article, ConfigState, SearchQuery};

use terraphim_middleware::run_ripgrep_service_and_index;
