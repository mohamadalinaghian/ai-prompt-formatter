use pfmt_core::{ValidationError, WriterOutput};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VerificationError {
    #[error("Schema validation failed: {0}")]
    Validation(#[from] ValidationError),
    #[error("Task brief is empty")]
    EmptyTask,
    #[error("Acceptance criteria must have between 1 and 8 items, found {0}")]
    InvalidAcceptanceCriteriaCount(usize),
}

pub struct Verifier;

impl Verifier {
    /// Validates and cleans WriterOutput against strict contracts:
    /// 1. Schema length checks (validate()).
    /// 2. Task non-empty.
    /// 3. Acceptance criteria has 1..=8 items.
    /// 4. Ensures `relevant_files` is strictly a subset of `candidates` (drops any hallucinations).
    pub fn verify_and_clean(
        mut output: WriterOutput,
        candidates: &[String],
    ) -> Result<WriterOutput, VerificationError> {
        // 1. Task non-empty
        if output.task.trim().is_empty() {
            return Err(VerificationError::EmptyTask);
        }

        // 2. Acceptance criteria count
        if output.acceptance_criteria.is_empty() || output.acceptance_criteria.len() > 8 {
            return Err(VerificationError::InvalidAcceptanceCriteriaCount(
                output.acceptance_criteria.len(),
            ));
        }

        // 3. Schema validation
        output.validate()?;

        // 4. Relevant files must be a subset of <candidates>; others dropped
        let original_count = output.relevant_files.len();
        output
            .relevant_files
            .retain(|file| candidates.contains(file));

        if output.relevant_files.len() < original_count {
            tracing::info!(
                "Verifier dropped {} hallucinated files not in candidates",
                original_count - output.relevant_files.len()
            );
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_drops_unlisted_files() {
        let output = WriterOutput {
            task: "Implement foo".to_string(),
            context: vec![],
            relevant_files: vec!["src/main.rs".to_string(), "src/hallucinated.rs".to_string()],
            acceptance_criteria: vec!["Works".to_string()],
            extra_rules: vec![],
            extra_report: vec![],
            assumptions: vec![],
        };

        let candidates = vec!["src/main.rs".to_string(), "Cargo.toml".to_string()];
        let verified = Verifier::verify_and_clean(output, &candidates).unwrap();

        assert_eq!(verified.relevant_files, vec!["src/main.rs"]);
    }

    #[test]
    fn test_verifier_rejects_empty_task() {
        let output = WriterOutput {
            task: "   ".to_string(),
            context: vec![],
            relevant_files: vec![],
            acceptance_criteria: vec!["Works".to_string()],
            extra_rules: vec![],
            extra_report: vec![],
            assumptions: vec![],
        };

        let res = Verifier::verify_and_clean(output, &[]);
        assert!(matches!(res, Err(VerificationError::EmptyTask)));
    }
}
