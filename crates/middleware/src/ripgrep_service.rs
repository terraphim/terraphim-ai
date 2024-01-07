
use terraphim_types::{ConfigState, SearchQuery, Article, merge_and_serialize};
use terraphim_pipeline::IndexedDocument; 
use std::collections::HashMap;

use terraphim_middleware::run_ripgrep_service_and_index;

