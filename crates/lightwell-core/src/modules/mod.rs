//! Tool modules: each one owns its descriptor, input parsing, state validation, no-op detection
//! and the compilation of its persisted payloads into host processing primitives. Modules never
//! write the catalog, never keep an undo stack and never render.
mod descriptor;
mod pixel;
mod processing;
mod registry;
mod transform;

pub use descriptor::{
    ActionDescriptor, Availability, CanvasInteraction, Control, EffectDescriptor, EffectStage,
    ModuleDescriptor, ParameterDescriptor, ParameterKind, check_parameters, valid_identity,
    valid_name,
};
pub use pixel::PixelModule;
pub use processing::{ExactGeometry, Processing, Stage};
pub use registry::ModuleRegistry;
pub use transform::TransformModule;

use crate::{Error, Layer};
use serde_json::{Map, Value};

/// A normalized action request: the durable history action identity and the parameter object
/// stored on the history entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionInput {
    pub action_id: String,
    pub parameters: Map<String, Value>,
}

/// What an action does to the current stack. A no-op records the request without a history row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionPlan {
    NoOp,
    Commit(Layer),
}

/// The current output stage and a point sampler over the current stack. The sampler evaluates one
/// pixel without rasterizing, so planning an action never allocates a frame.
pub struct StageContext<'a> {
    pub stage: Stage,
    #[allow(clippy::type_complexity)]
    pub sampler: &'a dyn Fn(u32, u32) -> Result<Option<[u8; 4]>, Error>,
}

pub trait ToolModule: Send + Sync {
    fn descriptor(&self) -> &ModuleDescriptor;
    /// Normalize an already schema-checked request into its durable action identity and stored
    /// parameters.
    fn parse(&self, action_id: &str, parameters: &Map<String, Value>)
    -> Result<ActionInput, Error>;
    /// Reject out-of-stage input and report no-ops against the current stack.
    fn plan(&self, input: &ActionInput, stage: &StageContext<'_>) -> Result<ActionPlan, Error>;
    /// Accept or reject a persisted payload structurally.
    fn validate_payload(&self, effect_id: &str, format: u32, payload: &Value) -> Result<(), Error>;
    /// Turn a persisted payload into a host processing primitive at its input stage.
    fn compile(
        &self,
        effect_id: &str,
        format: u32,
        payload: &Value,
        stage: Stage,
    ) -> Result<Processing, Error>;
}
