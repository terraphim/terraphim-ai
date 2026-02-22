use anyhow::{anyhow, Result};

/// Query AST node representing a parsed query
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryNode {
    /// Single concept: "BUN"
    Concept(String),
    /// AND: both must match
    And(Box<QueryNode>, Box<QueryNode>),
    /// OR: either matches
    Or(Box<QueryNode>, Box<QueryNode>),
    /// NOT: must not match
    Not(Box<QueryNode>),
}

/// Token types for lexical analysis
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Concept(String),
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
}

/// Tokenize a query string into tokens
fn tokenize(query: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars = query.chars();
    let mut current_word = String::new();

    for ch in chars {
        match ch {
            '(' => {
                if !current_word.is_empty() {
                    tokens.push(word_to_token(&current_word)?);
                    current_word.clear();
                }
                tokens.push(Token::LeftParen);
            }
            ')' => {
                if !current_word.is_empty() {
                    tokens.push(word_to_token(&current_word)?);
                    current_word.clear();
                }
                tokens.push(Token::RightParen);
            }
            ' ' | '\t' | '\n' | '\r' => {
                if !current_word.is_empty() {
                    tokens.push(word_to_token(&current_word)?);
                    current_word.clear();
                }
            }
            _ => {
                current_word.push(ch);
            }
        }
    }

    if !current_word.is_empty() {
        tokens.push(word_to_token(&current_word)?);
    }

    Ok(tokens)
}

/// Convert a word into a token
/// Keywords must be lowercase to be recognized as operators.
/// Mixed-case or uppercase variants are treated as concepts.
fn word_to_token(word: &str) -> Result<Token> {
    match word {
        "and" => Ok(Token::And),
        "or" => Ok(Token::Or),
        "not" => Ok(Token::Not),
        _ => Ok(Token::Concept(word.to_string())),
    }
}

/// Parser for boolean query expressions
pub struct QueryParser;

impl QueryParser {
    /// Parse a query string into a QueryNode AST
    ///
    /// # Errors
    ///
    /// Returns an error if the query is empty, has unmatched parentheses,
    /// or contains invalid syntax
    pub fn parse(query: &str) -> Result<QueryNode> {
        if query.trim().is_empty() {
            return Err(anyhow!("Query cannot be empty"));
        }

        let tokens = tokenize(query)?;
        if tokens.is_empty() {
            return Err(anyhow!("Query cannot be empty"));
        }

        let mut parser = ParserState {
            tokens,
            position: 0,
        };

        let node = parser.parse_expression()?;

        if parser.position < parser.tokens.len() {
            return Err(anyhow!("Unexpected token at position {}", parser.position));
        }

        Ok(node)
    }
}

/// Internal parser state
struct ParserState {
    tokens: Vec<Token>,
    position: usize,
}

impl ParserState {
    /// Parse a complete expression (handles OR at the top level)
    fn parse_expression(&mut self) -> Result<QueryNode> {
        let mut left = self.parse_and_expression()?;

        while self.position < self.tokens.len() {
            if let Some(Token::Or) = self.peek() {
                self.consume(); // consume OR
                let right = self.parse_and_expression()?;
                left = QueryNode::Or(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// Parse AND expressions (higher precedence than OR)
    fn parse_and_expression(&mut self) -> Result<QueryNode> {
        let mut left = self.parse_not_expression()?;

        while self.position < self.tokens.len() {
            if let Some(Token::And) = self.peek() {
                self.consume(); // consume AND
                let right = self.parse_not_expression()?;
                left = QueryNode::And(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// Parse NOT expressions and primary terms
    fn parse_not_expression(&mut self) -> Result<QueryNode> {
        if let Some(Token::Not) = self.peek() {
            self.consume(); // consume NOT
                            // Recursively parse NOT expressions to support "not not X"
            let operand = self.parse_not_expression()?;
            Ok(QueryNode::Not(Box::new(operand)))
        } else {
            self.parse_primary()
        }
    }

    /// Parse primary terms (concepts or parenthesized expressions)
    fn parse_primary(&mut self) -> Result<QueryNode> {
        match self.peek() {
            Some(Token::Concept(concept)) => {
                let concept = concept.clone();
                self.consume();
                Ok(QueryNode::Concept(concept))
            }
            Some(Token::LeftParen) => {
                self.consume(); // consume (
                let expr = self.parse_expression()?;
                match self.peek() {
                    Some(Token::RightParen) => {
                        self.consume(); // consume )
                        Ok(expr)
                    }
                    _ => Err(anyhow!("Expected closing parenthesis")),
                }
            }
            Some(token) => Err(anyhow!("Unexpected token: {:?}", token)),
            None => Err(anyhow!("Unexpected end of query")),
        }
    }

    /// Peek at the current token without consuming it
    fn peek(&self) -> Option<&Token> {
        if self.position < self.tokens.len() {
            Some(&self.tokens[self.position])
        } else {
            None
        }
    }

    /// Consume the current token and advance position
    fn consume(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_concept() {
        let query = "deploy";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), QueryNode::Concept("deploy".to_string()));
    }

    #[test]
    fn test_and_operator() {
        let query = "BUN and install";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Concept("BUN".to_string())),
                Box::new(QueryNode::Concept("install".to_string()))
            )
        );
    }

    #[test]
    fn test_or_operator() {
        let query = "deploy or publish";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::Or(
                Box::new(QueryNode::Concept("deploy".to_string())),
                Box::new(QueryNode::Concept("publish".to_string()))
            )
        );
    }

    #[test]
    fn test_not_operator() {
        let query = "deploy and not test";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Concept("deploy".to_string())),
                Box::new(QueryNode::Not(Box::new(QueryNode::Concept(
                    "test".to_string()
                ))))
            )
        );
    }

    #[test]
    fn test_nested_query() {
        let query = "wrangler and (deploy or publish)";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Concept("wrangler".to_string())),
                Box::new(QueryNode::Or(
                    Box::new(QueryNode::Concept("deploy".to_string())),
                    Box::new(QueryNode::Concept("publish".to_string()))
                ))
            )
        );
    }

    #[test]
    fn test_uppercase_keywords_are_concepts() {
        // Uppercase keywords should be treated as concepts, not operators
        // Only lowercase "and", "or", "not" are operators
        // When used with explicit lowercase operators, uppercase versions are concepts
        let query = "BUN and AND and install";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        // "AND" is a concept (not the operator), chained with lowercase "and" operators
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::And(
                    Box::new(QueryNode::Concept("BUN".to_string())),
                    Box::new(QueryNode::Concept("AND".to_string()))
                )),
                Box::new(QueryNode::Concept("install".to_string()))
            )
        );

        let query = "deploy or OR or publish";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        // "OR" is a concept (not the operator), chained with lowercase "or" operators
        assert_eq!(
            result.unwrap(),
            QueryNode::Or(
                Box::new(QueryNode::Or(
                    Box::new(QueryNode::Concept("deploy".to_string())),
                    Box::new(QueryNode::Concept("OR".to_string()))
                )),
                Box::new(QueryNode::Concept("publish".to_string()))
            )
        );

        // Uppercase NOT should be a concept
        let query = "test and NOT";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Concept("test".to_string())),
                Box::new(QueryNode::Concept("NOT".to_string()))
            )
        );
    }

    #[test]
    fn test_mixed_case_keywords_are_concepts() {
        // Regression test: mixed-case keywords like "oR" should be concepts
        // This was caught by proptest which generated "oR" and tried to use it in "oR or a"
        // Before the fix, "oR" was incorrectly treated as the OR operator

        // "oR" as a concept used with lowercase "or" operator
        let query = "oR or a";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        // "oR" is a concept (not an operator), combined with "a" via lowercase "or"
        assert_eq!(
            result.unwrap(),
            QueryNode::Or(
                Box::new(QueryNode::Concept("oR".to_string())),
                Box::new(QueryNode::Concept("a".to_string()))
            )
        );

        // "Or" as a concept
        let query = "Or and test";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Concept("Or".to_string())),
                Box::new(QueryNode::Concept("test".to_string()))
            )
        );

        // "aNd" as a concept
        let query = "aNd or test";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::Or(
                Box::new(QueryNode::Concept("aNd".to_string())),
                Box::new(QueryNode::Concept("test".to_string()))
            )
        );

        // "nOt" as a concept
        let query = "nOt and test";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Concept("nOt".to_string())),
                Box::new(QueryNode::Concept("test".to_string()))
            )
        );
    }

    #[test]
    fn test_complex_nested_query() {
        let query = "(rust or go) and (build or compile)";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Or(
                    Box::new(QueryNode::Concept("rust".to_string())),
                    Box::new(QueryNode::Concept("go".to_string()))
                )),
                Box::new(QueryNode::Or(
                    Box::new(QueryNode::Concept("build".to_string())),
                    Box::new(QueryNode::Concept("compile".to_string()))
                ))
            )
        );
    }

    #[test]
    fn test_not_with_parentheses() {
        let query = "deploy and not (test or staging)";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Concept("deploy".to_string())),
                Box::new(QueryNode::Not(Box::new(QueryNode::Or(
                    Box::new(QueryNode::Concept("test".to_string())),
                    Box::new(QueryNode::Concept("staging".to_string()))
                ))))
            )
        );
    }

    #[test]
    fn test_operator_precedence() {
        // AND has higher precedence than OR
        let query = "a or b and c";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        // Should parse as: a OR (b AND c)
        assert_eq!(
            result.unwrap(),
            QueryNode::Or(
                Box::new(QueryNode::Concept("a".to_string())),
                Box::new(QueryNode::And(
                    Box::new(QueryNode::Concept("b".to_string())),
                    Box::new(QueryNode::Concept("c".to_string()))
                ))
            )
        );
    }

    #[test]
    fn test_multiple_ands() {
        let query = "a and b and c";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::And(
                    Box::new(QueryNode::Concept("a".to_string())),
                    Box::new(QueryNode::Concept("b".to_string()))
                )),
                Box::new(QueryNode::Concept("c".to_string()))
            )
        );
    }

    #[test]
    fn test_multiple_ors() {
        let query = "a or b or c";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::Or(
                Box::new(QueryNode::Or(
                    Box::new(QueryNode::Concept("a".to_string())),
                    Box::new(QueryNode::Concept("b".to_string()))
                )),
                Box::new(QueryNode::Concept("c".to_string()))
            )
        );
    }

    #[test]
    fn test_empty_query() {
        let query = "";
        let result = QueryParser::parse(query);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_whitespace_query() {
        let query = "   ";
        let result = QueryParser::parse(query);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_unmatched_left_paren() {
        let query = "(deploy and publish";
        let result = QueryParser::parse(query);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("closing parenthesis"));
    }

    #[test]
    fn test_unmatched_right_paren() {
        let query = "deploy and publish)";
        let result = QueryParser::parse(query);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unexpected token"));
    }

    #[test]
    fn test_concepts_with_special_chars() {
        // Concepts can contain various characters
        let query = "rust-analyzer and cargo-build";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Concept("rust-analyzer".to_string())),
                Box::new(QueryNode::Concept("cargo-build".to_string()))
            )
        );
    }

    #[test]
    fn test_whitespace_handling() {
        let query = "  deploy   and   publish  ";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Concept("deploy".to_string())),
                Box::new(QueryNode::Concept("publish".to_string()))
            )
        );
    }

    #[test]
    fn test_nested_parentheses() {
        let query = "((a or b) and (c or d))";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::And(
                Box::new(QueryNode::Or(
                    Box::new(QueryNode::Concept("a".to_string())),
                    Box::new(QueryNode::Concept("b".to_string()))
                )),
                Box::new(QueryNode::Or(
                    Box::new(QueryNode::Concept("c".to_string())),
                    Box::new(QueryNode::Concept("d".to_string()))
                ))
            )
        );
    }

    #[test]
    fn test_not_at_start() {
        let query = "not test";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::Not(Box::new(QueryNode::Concept("test".to_string())))
        );
    }

    #[test]
    fn test_double_not() {
        let query = "not not deploy";
        let result = QueryParser::parse(query);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            QueryNode::Not(Box::new(QueryNode::Not(Box::new(QueryNode::Concept(
                "deploy".to_string()
            )))))
        );
    }

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        // Strategy to generate valid concept names (excluding reserved keywords)
        fn concept_strategy() -> impl Strategy<Value = String> {
            "[a-zA-Z][a-zA-Z0-9_-]{0,20}".prop_filter("must not be reserved keyword", |s| {
                let lower = s.to_lowercase();
                lower != "and" && lower != "or" && lower != "not"
            })
        }

        proptest! {
            #[test]
            fn test_single_concept_always_parses(concept in concept_strategy()) {
                let result = QueryParser::parse(&concept);
                prop_assert!(result.is_ok());
                prop_assert_eq!(result.unwrap(), QueryNode::Concept(concept));
            }

            #[test]
            fn test_and_query_parses(
                left in concept_strategy(),
                right in concept_strategy()
            ) {
                let query = format!("{} and {}", left, right);
                let result = QueryParser::parse(&query);
                prop_assert!(result.is_ok());
                prop_assert_eq!(
                    result.unwrap(),
                    QueryNode::And(
                        Box::new(QueryNode::Concept(left)),
                        Box::new(QueryNode::Concept(right))
                    )
                );
            }

            #[test]
            fn test_or_query_parses(
                left in concept_strategy(),
                right in concept_strategy()
            ) {
                let query = format!("{} or {}", left, right);
                let result = QueryParser::parse(&query);
                prop_assert!(result.is_ok());
                prop_assert_eq!(
                    result.unwrap(),
                    QueryNode::Or(
                        Box::new(QueryNode::Concept(left)),
                        Box::new(QueryNode::Concept(right))
                    )
                );
            }

            #[test]
            fn test_not_query_parses(concept in concept_strategy()) {
                let query = format!("not {}", concept);
                let result = QueryParser::parse(&query);
                prop_assert!(result.is_ok());
                prop_assert_eq!(
                    result.unwrap(),
                    QueryNode::Not(Box::new(QueryNode::Concept(concept)))
                );
            }

            #[test]
            fn test_parenthesized_query_parses(concept in concept_strategy()) {
                let query = format!("({})", concept);
                let result = QueryParser::parse(&query);
                prop_assert!(result.is_ok());
                prop_assert_eq!(result.unwrap(), QueryNode::Concept(concept));
            }
        }
    }
}
