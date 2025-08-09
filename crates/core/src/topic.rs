use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A simple data structure to hold the state of a single subtopic.
/// The `states` (e.g., has_definition) are now managed by the LLM
/// and updated via tool calls.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubTopic {
    pub name: String,
    pub has_definition: bool,
    pub has_mechanism: bool,
    pub has_example: bool,
}

impl SubTopic {
    pub fn new(name: String) -> Self {
        Self {
            name,
            has_definition: false,
            has_mechanism: false,
            has_example: false,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.has_definition && self.has_mechanism && self.has_example
    }
}
