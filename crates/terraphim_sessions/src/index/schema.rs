//! Tantivy schema definition for session indexing

use tantivy::schema::*;

/// Field name constants
pub const FIELD_ID: &str = "id";
pub const FIELD_TITLE: &str = "title";
pub const FIELD_BODY: &str = "body";
pub const FIELD_SOURCE: &str = "source";
pub const FIELD_ROLE: &str = "role";
pub const FIELD_TIMESTAMP: &str = "timestamp";
pub const FIELD_TAGS: &str = "tags";

/// Build the Tantivy schema for session documents
pub fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // ID field - stored, indexed for exact lookup
    schema_builder.add_text_field(FIELD_ID, TEXT | STORED);

    // Title - tokenized, stored, with position info for phrase queries
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("default")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();
    schema_builder.add_text_field(FIELD_TITLE, text_options);

    // Body - tokenized, not stored (reconstructed from session), with positions
    let body_options = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("default")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    schema_builder.add_text_field(FIELD_BODY, body_options);

    // Source - stored, indexed for filtering
    schema_builder.add_text_field(FIELD_SOURCE, TEXT | STORED);

    // Role - stored, indexed for filtering
    schema_builder.add_text_field(FIELD_ROLE, TEXT | STORED);

    // Timestamp - fast field for sorting/range queries
    schema_builder.add_date_field(FIELD_TIMESTAMP, FAST | STORED);

    // Tags - tokenized for search, stored
    schema_builder.add_text_field(FIELD_TAGS, TEXT | STORED);

    schema_builder.build()
}

/// Global schema instance (lazy)
pub static SESSION_SCHEMA: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();

/// Get or initialize the global schema
pub fn get_schema() -> &'static Schema {
    SESSION_SCHEMA.get_or_init(build_schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_has_required_fields() {
        let schema = build_schema();
        assert!(schema.get_field(FIELD_ID).is_ok());
        assert!(schema.get_field(FIELD_TITLE).is_ok());
        assert!(schema.get_field(FIELD_BODY).is_ok());
        assert!(schema.get_field(FIELD_SOURCE).is_ok());
        assert!(schema.get_field(FIELD_ROLE).is_ok());
        assert!(schema.get_field(FIELD_TIMESTAMP).is_ok());
        assert!(schema.get_field(FIELD_TAGS).is_ok());
    }
}
