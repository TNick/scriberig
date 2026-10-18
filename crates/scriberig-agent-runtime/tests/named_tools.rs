//! Verifies named provider-facing control tools in the agent loop.

use std::sync::atomic::AtomicBool;

use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_agent_runtime::{AgentMessage, AgentRuntime, ModelDecision, ModelProvider};
use scriberig_app::ApplicationContext;
use scriberig_core::EntityId;

/// Accepts a named control call and returns a bounded result.
struct ControlTool;

impl Tool for ControlTool {
    /// Returns the fixture tool identifier.
    ///
    /// # Returns
    /// The stable control-tool identifier.
    fn name(&self) -> &'static str {
        "control"
    }

    /// Rejects ordinary entity inspection for this control-only fixture.
    ///
    /// # Parameters
    /// - `_context`: Runtime application context.
    /// - `_input`: Entity inspection input.
    ///
    /// # Returns
    /// Never returns a successful inspection.
    ///
    /// # Errors
    /// Always returns unsupported-operation.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::Application {
            category: "unsupported_operation".to_owned(),
            message: "inspection is not configured".to_owned(),
        })
    }

    /// Executes the named control fixture.
    ///
    /// # Parameters
    /// - `_context`: Runtime application context.
    /// - `name`: Named control tool.
    /// - `_arguments`: Control arguments.
    ///
    /// # Returns
    /// A bounded control result.
    ///
    /// # Errors
    /// This fixture rejects no named tool.
    fn invoke_named(
        &self,
        _context: &ApplicationContext,
        name: &str,
        _arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult {
            entity: EntityId::from_bytes([0; 16]),
            object_type: Some(name.to_owned()),
            revision: None,
            snapshot: None,
            summary: Some("control complete".to_owned()),
            handles: vec![name.to_owned()],
        })
    }
}

/// Verifies named agent controls use the same bounded runtime tool loop.
#[test]
fn runtime_executes_named_control_tools() {
    /// Provider fixture requesting a named control tool once.
    struct Provider;

    impl ModelProvider for Provider {
        /// Returns a control decision followed by a final response.
        ///
        /// # Parameters
        /// - `messages`: Current runtime messages.
        ///
        /// # Returns
        /// A named control call or final response.
        ///
        /// # Errors
        /// This fixture never fails.
        fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
            if messages.len() == 1 {
                return Ok(ModelDecision::NamedTool {
                    name: "operation_status".to_owned(),
                    arguments: serde_json::json!({"operation": "op-1"}),
                    presentation_arguments: serde_json::json!({"operation": "op-1"}),
                    presentation_label: None,
                });
            }
            assert!(matches!(messages[1], AgentMessage::Tool(_)));
            Ok(ModelDecision::Final("done".to_owned()))
        }
    }

    let result = AgentRuntime::new(Provider, ControlTool, 2)
        .run(
            &ApplicationContext::empty(),
            "observe",
            &AtomicBool::new(false),
        )
        .expect("named control runtime");
    assert_eq!(result.output, "done");
    let AgentMessage::Tool(tool) = &result.messages[1] else {
        panic!("expected named control result");
    };
    assert_eq!(tool.summary.as_deref(), Some("control complete"));
}
