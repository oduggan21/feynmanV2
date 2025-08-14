use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A data structure to hold the state of a single subtopic.
///
/// The learning state for each criterion (e.g., `has_definition`) is managed
/// by the LLM and updated via tool calls to the `FeynmanAgent`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubTopic {
    pub name: String,
    pub has_definition: bool,
    pub has_mechanism: bool,
    pub has_example: bool,
}

impl SubTopic {
    /// Creates a new, incomplete `SubTopic`.
    pub fn new(name: String) -> Self {
        Self {
            name,
            has_definition: false,
            has_mechanism: false,
            has_example: false,
        }
    }

    /// Checks if the subtopic is fully covered across all criteria.
    pub fn is_complete(&self) -> bool {
        self.has_definition && self.has_mechanism && self.has_example
    }
}
