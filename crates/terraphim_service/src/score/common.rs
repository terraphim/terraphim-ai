/// Common structures shared across different BM25 scoring implementations
/// 
/// This module contains shared structs and utilities used by various BM25 
/// scoring algorithms to avoid code duplication and ensure consistency.

/// BM25 parameters used across different BM25 implementations
/// 
/// These parameters control the behavior of BM25 family of scoring algorithms:
/// - k1: Controls term frequency saturation point
/// - b: Controls document length normalization strength
/// - delta: Used by BM25+ to address the lower-bounding problem
#[derive(Debug, Clone, PartialEq)]
pub struct BM25Params {
    /// k1 parameter controls term frequency saturation
    pub k1: f64,
    /// b parameter controls document length normalization
    pub b: f64,
    /// delta parameter for BM25+ to address the lower-bounding problem
    pub delta: f64,
}

impl Default for BM25Params {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            delta: 1.0,
        }
    }
}

/// Field weights for BM25F field-based scoring
/// 
/// Different document fields can have different importance weights:
/// - title: Usually highest weight as titles are most important
/// - body: Base weight for main content
/// - description: Medium weight for summaries
/// - tags: High weight for categorical information
#[derive(Debug, Clone, PartialEq)]
pub struct FieldWeights {
    /// Weight for document title
    pub title: f64,
    /// Weight for document body
    pub body: f64,
    /// Weight for document description (if available)
    pub description: f64,
    /// Weight for document tags (if available)
    pub tags: f64,
}

impl Default for FieldWeights {
    fn default() -> Self {
        Self {
            title: 3.0,
            body: 1.0,
            description: 2.0,
            tags: 2.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_params_default() {
        let params = BM25Params::default();
        assert_eq!(params.k1, 1.2);
        assert_eq!(params.b, 0.75);
        assert_eq!(params.delta, 1.0);
    }

    #[test]
    fn test_field_weights_default() {
        let weights = FieldWeights::default();
        assert_eq!(weights.title, 3.0);
        assert_eq!(weights.body, 1.0);
        assert_eq!(weights.description, 2.0);
        assert_eq!(weights.tags, 2.5);
    }

    #[test]
    fn test_bm25_params_custom() {
        let params = BM25Params {
            k1: 1.5,
            b: 0.8,
            delta: 0.5,
        };
        assert_eq!(params.k1, 1.5);
        assert_eq!(params.b, 0.8);
        assert_eq!(params.delta, 0.5);
    }

    #[test]
    fn test_field_weights_custom() {
        let weights = FieldWeights {
            title: 4.0,
            body: 1.5,
            description: 2.5,
            tags: 3.0,
        };
        assert_eq!(weights.title, 4.0);
        assert_eq!(weights.body, 1.5);
        assert_eq!(weights.description, 2.5);
        assert_eq!(weights.tags, 3.0);
    }
}