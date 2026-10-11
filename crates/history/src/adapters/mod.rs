pub mod antigravity;
pub mod freebuff;
pub mod hermes;
pub mod opencode;

pub use antigravity::AntigravityAdapter;
pub use freebuff::FreebuffAdapter;
pub use hermes::HermesAdapter;
pub use opencode::OpenCodeAdapter;

use crate::adapter::AgentAdapter;

pub fn default_adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(AntigravityAdapter::default()),
        Box::new(OpenCodeAdapter::default()),
        Box::new(HermesAdapter::default()),
        Box::new(FreebuffAdapter::default()),
    ]
}
