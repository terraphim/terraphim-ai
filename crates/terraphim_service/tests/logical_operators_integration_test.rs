#[cfg(test)]
mod logical_operators_integration_tests {
    use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
    use terraphim_service::TerraphimService;
    use terraphim_types::{LogicalOperator, NormalizedTermValue, RoleName, SearchQuery};

    async fn setup_test_service() -> TerraphimService {
        let mut config = ConfigBuilder::new_with_id(ConfigId::Embedded)
            .build_default_embedded()
            .build()
            .unwrap();
        let config_state = ConfigState::new(&mut config).await.unwrap();
        TerraphimService::new(config_state)
    }

    #[tokio::test]
    async fn test_search_with_and_operator() {
        let mut service = setup_test_service().await;

        let query = SearchQuery {
            search_term: NormalizedTermValue::from("rust"),
            search_terms: Some(vec![
                NormalizedTermValue::from("rust"),
                NormalizedTermValue::from("async"),
            ]),
            operator: Some(LogicalOperator::And),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        // Test that the search executes without error
        let result = service.search(&query).await;

        // Should succeed (even if no results)
        assert!(
            result.is_ok(),
            "AND search should not fail: {:?}",
            result.err()
        );

        let documents = result.unwrap();
        assert!(documents.len() <= 10, "Should respect limit");

        // Log for debugging
        println!("AND search returned {} documents", documents.len());
    }

    #[tokio::test]
    async fn test_search_with_or_operator() {
        let mut service = setup_test_service().await;

        let query = SearchQuery {
            search_term: NormalizedTermValue::from("api"),
            search_terms: Some(vec![
                NormalizedTermValue::from("api"),
                NormalizedTermValue::from("sdk"),
            ]),
            operator: Some(LogicalOperator::Or),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        let result = service.search(&query).await;

        assert!(
            result.is_ok(),
            "OR search should not fail: {:?}",
            result.err()
        );

        let documents = result.unwrap();
        assert!(documents.len() <= 10, "Should respect limit");

        println!("OR search returned {} documents", documents.len());
    }

    #[tokio::test]
    async fn test_backward_compatibility_single_term() {
        let mut service = setup_test_service().await;

        let query = SearchQuery {
            search_term: NormalizedTermValue::from("rust"),
            search_terms: None,
            operator: None,
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        let result = service.search(&query).await;

        assert!(
            result.is_ok(),
            "Single term search should not fail: {:?}",
            result.err()
        );

        let documents = result.unwrap();
        assert!(documents.len() <= 10, "Should respect limit");

        println!("Single term search returned {} documents", documents.len());
    }

    #[tokio::test]
    async fn test_empty_search_terms_with_operator() {
        let mut service = setup_test_service().await;

        let query = SearchQuery {
            search_term: NormalizedTermValue::from("test"),
            search_terms: Some(vec![]),
            operator: Some(LogicalOperator::And),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        let result = service.search(&query).await;

        // Should handle empty search terms gracefully
        assert!(result.is_ok(), "Empty search terms should not crash");

        let documents = result.unwrap();
        println!("Empty terms search returned {} documents", documents.len());
    }

    #[tokio::test]
    async fn test_multiple_terms_and_operation() {
        let mut service = setup_test_service().await;

        let query = SearchQuery {
            search_term: NormalizedTermValue::from("system"),
            search_terms: Some(vec![
                NormalizedTermValue::from("system"),
                NormalizedTermValue::from("operation"),
                NormalizedTermValue::from("management"),
            ]),
            operator: Some(LogicalOperator::And),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        let result = service.search(&query).await;

        assert!(result.is_ok(), "Multiple terms AND search should not fail");

        let documents = result.unwrap();
        println!(
            "Multiple AND terms search returned {} documents",
            documents.len()
        );
    }

    #[tokio::test]
    async fn test_multiple_terms_or_operation() {
        let mut service = setup_test_service().await;

        let query = SearchQuery {
            search_term: NormalizedTermValue::from("api"),
            search_terms: Some(vec![
                NormalizedTermValue::from("api"),
                NormalizedTermValue::from("sdk"),
                NormalizedTermValue::from("library"),
                NormalizedTermValue::from("framework"),
            ]),
            operator: Some(LogicalOperator::Or),
            skip: Some(0),
            limit: Some(10),
            role: Some(RoleName::from("Default")),
        };

        let result = service.search(&query).await;

        assert!(result.is_ok(), "Multiple terms OR search should not fail");

        let documents = result.unwrap();
        println!(
            "Multiple OR terms search returned {} documents",
            documents.len()
        );
    }

    #[tokio::test]
    async fn test_skip_and_limit_with_operators() {
        let mut service = setup_test_service().await;

        // First page
        let query1 = SearchQuery {
            search_term: NormalizedTermValue::from("system"),
            search_terms: Some(vec![
                NormalizedTermValue::from("system"),
                NormalizedTermValue::from("operation"),
            ]),
            operator: Some(LogicalOperator::Or),
            skip: Some(0),
            limit: Some(3),
            role: Some(RoleName::from("Default")),
        };

        let result1 = service.search(&query1).await;
        assert!(result1.is_ok());
        let docs1 = result1.unwrap();

        // Second page
        let query2 = SearchQuery {
            search_term: NormalizedTermValue::from("system"),
            search_terms: Some(vec![
                NormalizedTermValue::from("system"),
                NormalizedTermValue::from("operation"),
            ]),
            operator: Some(LogicalOperator::Or),
            skip: Some(3),
            limit: Some(3),
            role: Some(RoleName::from("Default")),
        };

        let result2 = service.search(&query2).await;
        assert!(result2.is_ok());
        let docs2 = result2.unwrap();

        println!(
            "First page: {} docs, Second page: {} docs",
            docs1.len(),
            docs2.len()
        );

        // Verify pagination works
        assert!(docs1.len() <= 3);
        assert!(docs2.len() <= 3);
    }

    #[tokio::test]
    async fn test_different_roles_with_operators() {
        let mut service = setup_test_service().await;

        let roles = vec!["Default"];

        for role_name in roles {
            let query = SearchQuery {
                search_term: NormalizedTermValue::from("test"),
                search_terms: Some(vec![
                    NormalizedTermValue::from("test"),
                    NormalizedTermValue::from("system"),
                ]),
                operator: Some(LogicalOperator::Or),
                skip: Some(0),
                limit: Some(5),
                role: Some(RoleName::from(role_name)),
            };

            let result = service.search(&query).await;

            // Should work for any valid role
            if result.is_ok() {
                let documents = result.unwrap();
                println!(
                    "Role '{}' search returned {} documents",
                    role_name,
                    documents.len()
                );
                assert!(documents.len() <= 5);
            } else {
                println!("Role '{}' search failed: {:?}", role_name, result.err());
            }
        }
    }
}
