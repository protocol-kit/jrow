//! NATS-style pattern matching for topics
//!
//! This module implements pattern matching compatible with NATS messaging system.
//! Topics are split into tokens using `.` as a delimiter, and wildcards match tokens.
//!
//! # Wildcard Semantics
//!
//! - `*` - Matches exactly one token at that position
//! - `>` - Matches one or more tokens (must be last token)
//!
//! # Pattern Rules
//!
//! - Wildcards are token-level, not character-level
//! - `*` and `>` cannot be mixed in the same pattern
//! - `>` must be the last token if present
//! - Empty tokens (consecutive `.`) are not allowed
//!
//! # Examples
//!
//! ```rust
//! use jrow_server::NatsPattern;
//!
//! // Exact match
//! let exact = NatsPattern::new("orders.created").unwrap();
//! assert!(exact.matches("orders.created"));
//! assert!(!exact.matches("orders.updated"));
//!
//! // Single wildcard - one token
//! let pattern = NatsPattern::new("orders.*.completed").unwrap();
//! assert!(pattern.matches("orders.123.completed"));
//! assert!(pattern.matches("orders.456.completed"));
//! assert!(!pattern.matches("orders.123.456.completed")); // too many tokens
//!
//! // Multi wildcard - any trailing tokens
//! let multi = NatsPattern::new("events.>").unwrap();
//! assert!(multi.matches("events.user"));
//! assert!(multi.matches("events.user.login"));
//! assert!(multi.matches("events.user.login.success"));
//! ```
//!
//! # Performance
//!
//! Pattern matching is optimized for common cases:
//! - Exact matches use simple string comparison
//! - Single wildcards do token-by-token comparison
//! - Multi wildcards only check prefix tokens

use std::fmt;

/// Error type for pattern parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// Empty pattern string
    EmptyPattern,
    /// Empty token (e.g., "orders..new")
    EmptyToken,
    /// Wildcard combined with text (e.g., "ord*")
    CombinedWildcard,
    /// Multi-token wildcard not at end (e.g., "orders.>.new")
    MultiWildcardNotLast,
    /// Pattern contains both * and >
    MixedWildcards,
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternError::EmptyPattern => write!(f, "Pattern cannot be empty"),
            PatternError::EmptyToken => write!(f, "Pattern contains empty token (consecutive dots)"),
            PatternError::CombinedWildcard => write!(f, "Wildcard cannot be combined with text in the same token"),
            PatternError::MultiWildcardNotLast => write!(f, "Multi-token wildcard '>' must be the last token"),
            PatternError::MixedWildcards => write!(f, "Pattern cannot contain both '*' and '>' wildcards"),
        }
    }
}

impl std::error::Error for PatternError {}

/// A token in a NATS pattern
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Literal string token
    Literal(String),
    /// Single-token wildcard (*)
    SingleWild,
    /// Multi-token wildcard (>)
    MultiWild,
}

/// NATS-style pattern for topic matching
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NatsPattern {
    /// Exact topic match (no wildcards)
    Exact(String),
    /// Pattern with single-token wildcards (*)
    SingleWildcard { 
        pattern: String,
        tokens: Vec<Token>,
    },
    /// Pattern with multi-token wildcard (>) at the end
    MultiWildcard { 
        pattern: String,
        prefix: Vec<Token>,
    },
}

impl NatsPattern {
    /// Create a new pattern from a string
    ///
    /// # Examples
    ///
    /// ```
    /// use jrow_server::NatsPattern;
    ///
    /// let exact = NatsPattern::new("orders").unwrap();
    /// let single = NatsPattern::new("orders.*.shipped").unwrap();
    /// let multi = NatsPattern::new("orders.>").unwrap();
    /// ```
    pub fn new(pattern: &str) -> Result<Self, PatternError> {
        if pattern.is_empty() {
            return Err(PatternError::EmptyPattern);
        }

        // Check for wildcards
        let has_single_wild = pattern.contains('*');
        let has_multi_wild = pattern.contains('>');

        // Cannot mix wildcard types
        if has_single_wild && has_multi_wild {
            return Err(PatternError::MixedWildcards);
        }

        // No wildcards - exact match
        if !has_single_wild && !has_multi_wild {
            // Validate no empty tokens
            if pattern.contains("..") {
                return Err(PatternError::EmptyToken);
            }
            return Ok(NatsPattern::Exact(pattern.to_string()));
        }

        // Parse tokens
        let parts: Vec<&str> = pattern.split('.').collect();
        let mut tokens = Vec::new();

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                return Err(PatternError::EmptyToken);
            }

            if *part == "*" {
                tokens.push(Token::SingleWild);
            } else if *part == ">" {
                // Multi-wildcard must be last token
                if i != parts.len() - 1 {
                    return Err(PatternError::MultiWildcardNotLast);
                }
                tokens.push(Token::MultiWild);
            } else {
                // Check for wildcards combined with text
                if part.contains('*') || part.contains('>') {
                    return Err(PatternError::CombinedWildcard);
                }
                tokens.push(Token::Literal(part.to_string()));
            }
        }

        if has_multi_wild {
            // Remove the last token (which is MultiWild)
            tokens.pop();
            Ok(NatsPattern::MultiWildcard {
                pattern: pattern.to_string(),
                prefix: tokens,
            })
        } else {
            Ok(NatsPattern::SingleWildcard {
                pattern: pattern.to_string(),
                tokens,
            })
        }
    }

    /// Check if a topic matches this pattern
    ///
    /// # Examples
    ///
    /// ```
    /// use jrow_server::NatsPattern;
    ///
    /// let pattern = NatsPattern::new("orders.*.shipped").unwrap();
    /// assert!(pattern.matches("orders.new.shipped"));
    /// assert!(!pattern.matches("orders.shipped"));
    /// ```
    pub fn matches(&self, topic: &str) -> bool {
        match self {
            NatsPattern::Exact(exact) => exact == topic,
            NatsPattern::SingleWildcard { tokens, .. } => {
                Self::matches_single_wildcard(tokens, topic)
            }
            NatsPattern::MultiWildcard { prefix, .. } => {
                Self::matches_multi_wildcard(prefix, topic)
            }
        }
    }

    /// Check if this is a pattern (has wildcards)
    pub fn is_pattern(&self) -> bool {
        !matches!(self, NatsPattern::Exact(_))
    }

    /// Get the original pattern string
    pub fn as_str(&self) -> &str {
        match self {
            NatsPattern::Exact(s) => s,
            NatsPattern::SingleWildcard { pattern, .. } => pattern,
            NatsPattern::MultiWildcard { pattern, .. } => pattern,
        }
    }

    /// Match topic against single-wildcard pattern
    fn matches_single_wildcard(tokens: &[Token], topic: &str) -> bool {
        let topic_parts: Vec<&str> = topic.split('.').collect();

        // Must have same number of tokens
        if tokens.len() != topic_parts.len() {
            return false;
        }

        // Match token by token
        for (token, topic_part) in tokens.iter().zip(topic_parts.iter()) {
            match token {
                Token::Literal(lit) => {
                    if lit != topic_part {
                        return false;
                    }
                }
                Token::SingleWild => {
                    // Matches any single token
                    continue;
                }
                Token::MultiWild => {
                    // Should not happen in single wildcard pattern
                    return false;
                }
            }
        }

        true
    }

    /// Match topic against multi-wildcard pattern
    fn matches_multi_wildcard(prefix: &[Token], topic: &str) -> bool {
        let topic_parts: Vec<&str> = topic.split('.').collect();

        // Topic must have MORE tokens than prefix (> matches one or more additional tokens)
        if topic_parts.len() <= prefix.len() {
            return false;
        }

        // Match prefix tokens
        for (token, topic_part) in prefix.iter().zip(topic_parts.iter()) {
            match token {
                Token::Literal(lit) => {
                    if lit != topic_part {
                        return false;
                    }
                }
                Token::SingleWild => {
                    // Matches any single token
                    continue;
                }
                Token::MultiWild => {
                    // Should not happen in prefix
                    return false;
                }
            }
        }

        // If we get here, prefix matched and > matches remaining tokens
        true
    }
}

impl fmt::Display for NatsPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_pattern() {
        let pattern = NatsPattern::new("orders").unwrap();
        assert!(matches!(pattern, NatsPattern::Exact(_)));
        assert!(!pattern.is_pattern());
        assert_eq!(pattern.as_str(), "orders");

        assert!(pattern.matches("orders"));
        assert!(!pattern.matches("orders.new"));
        assert!(!pattern.matches("order"));
    }

    #[test]
    fn test_exact_multi_token() {
        let pattern = NatsPattern::new("orders.new.shipped").unwrap();
        assert!(matches!(pattern, NatsPattern::Exact(_)));

        assert!(pattern.matches("orders.new.shipped"));
        assert!(!pattern.matches("orders.new"));
        assert!(!pattern.matches("orders.old.shipped"));
    }

    #[test]
    fn test_single_wildcard_one_token() {
        let pattern = NatsPattern::new("orders.*").unwrap();
        assert!(pattern.is_pattern());

        assert!(pattern.matches("orders.new"));
        assert!(pattern.matches("orders.old"));
        assert!(pattern.matches("orders.shipped"));
        assert!(!pattern.matches("orders"));
        assert!(!pattern.matches("orders.new.shipped"));
    }

    #[test]
    fn test_single_wildcard_middle() {
        let pattern = NatsPattern::new("orders.*.shipped").unwrap();

        assert!(pattern.matches("orders.new.shipped"));
        assert!(pattern.matches("orders.old.shipped"));
        assert!(!pattern.matches("orders.shipped"));
        assert!(!pattern.matches("orders.new.pending.shipped"));
    }

    #[test]
    fn test_single_wildcard_beginning() {
        let pattern = NatsPattern::new("*.new").unwrap();

        assert!(pattern.matches("orders.new"));
        assert!(pattern.matches("events.new"));
        assert!(!pattern.matches("orders.old"));
        assert!(!pattern.matches("orders.new.shipped"));
    }

    #[test]
    fn test_multiple_single_wildcards() {
        let pattern = NatsPattern::new("orders.*.*").unwrap();

        assert!(pattern.matches("orders.new.shipped"));
        assert!(pattern.matches("orders.old.pending"));
        assert!(!pattern.matches("orders.new"));
        assert!(!pattern.matches("orders.new.pending.shipped"));
    }

    #[test]
    fn test_multi_wildcard_simple() {
        let pattern = NatsPattern::new("orders.>").unwrap();
        assert!(pattern.is_pattern());

        assert!(pattern.matches("orders.new"));
        assert!(pattern.matches("orders.new.shipped"));
        assert!(pattern.matches("orders.new.pending.shipped"));
        assert!(!pattern.matches("orders"));
        assert!(!pattern.matches("events.new"));
    }

    #[test]
    fn test_multi_wildcard_with_prefix() {
        let pattern = NatsPattern::new("orders.new.>").unwrap();

        assert!(pattern.matches("orders.new.shipped"));
        assert!(pattern.matches("orders.new.pending.shipped"));
        assert!(!pattern.matches("orders.new"));
        assert!(!pattern.matches("orders.old.shipped"));
    }

    #[test]
    fn test_multi_wildcard_root() {
        let pattern = NatsPattern::new(">").unwrap();

        assert!(pattern.matches("orders"));
        assert!(pattern.matches("orders.new"));
        assert!(pattern.matches("orders.new.shipped"));
    }

    #[test]
    fn test_empty_pattern() {
        let result = NatsPattern::new("");
        assert!(matches!(result, Err(PatternError::EmptyPattern)));
    }

    #[test]
    fn test_empty_token() {
        let result = NatsPattern::new("orders..new");
        assert!(matches!(result, Err(PatternError::EmptyToken)));
    }

    #[test]
    fn test_combined_wildcard() {
        let result = NatsPattern::new("ord*");
        assert!(matches!(result, Err(PatternError::CombinedWildcard)));

        let result = NatsPattern::new("orders.new*");
        assert!(matches!(result, Err(PatternError::CombinedWildcard)));
    }

    #[test]
    fn test_multi_wildcard_not_last() {
        let result = NatsPattern::new("orders.>.new");
        assert!(matches!(result, Err(PatternError::MultiWildcardNotLast)));
    }

    #[test]
    fn test_mixed_wildcards() {
        let result = NatsPattern::new("orders.*.>");
        assert!(matches!(result, Err(PatternError::MixedWildcards)));
    }

    #[test]
    fn test_pattern_display() {
        let pattern = NatsPattern::new("orders.*.shipped").unwrap();
        assert_eq!(format!("{}", pattern), "orders.*.shipped");
    }

    #[test]
    fn test_complex_patterns() {
        // Three wildcards
        let pattern = NatsPattern::new("*.*.*").unwrap();
        assert!(pattern.matches("orders.new.shipped"));
        assert!(!pattern.matches("orders.new"));

        // Mix of literal and wildcard
        let pattern = NatsPattern::new("orders.*.shipped.*").unwrap();
        assert!(pattern.matches("orders.new.shipped.fast"));
        assert!(!pattern.matches("orders.new.shipped"));
    }

    #[test]
    fn test_edge_cases() {
        // Single token exact
        let pattern = NatsPattern::new("orders").unwrap();
        assert!(pattern.matches("orders"));
        assert!(!pattern.matches("orders.new"));

        // Single wildcard alone
        let pattern = NatsPattern::new("*").unwrap();
        assert!(pattern.matches("orders"));
        assert!(pattern.matches("events"));
        assert!(!pattern.matches("orders.new"));
    }
}

