//! Completes connection exercises when the platform's MCP gateway observes an
//! authenticated tool call for the learner's session. The gateway only emits
//! events after verifying the session-scoped token, so the event itself is the
//! evidence.

use async_trait::async_trait;
use lp_core::{
    Capability, CheckResult, CompletionStrategy, Extension, ExtensionEvent, ExtensionManifest,
    StepCategory, StepContext, StepTypeSpec, ValidationOutcome, parse_config,
};
use serde::Deserialize;
use serde_json::Value;

use crate::support::{collect_issues, unsupported_step_type};

pub const ID: &str = "mcp";
pub const TOOL_CALLED: &str = "mcp.tool_called";
const STEP_TYPE: &str = "mcp_connection_check";

pub struct McpExtension {
    manifest: ExtensionManifest,
}

impl McpExtension {
    pub fn new() -> Self {
        Self {
            manifest: ExtensionManifest {
                id: ID,
                name: "Model Context Protocol",
                version: "1.0.0",
                description: "Detects AI agents connecting to the platform's MCP server.",
                capabilities: vec![Capability::ConfigValidation, Capability::EventHandling],
                step_types: vec![StepTypeSpec::new(
                    STEP_TYPE,
                    StepCategory::Practical,
                    CompletionStrategy::Event,
                    "Completes when the learner's agent calls the platform over MCP.",
                )],
                event_kinds: vec![TOOL_CALLED],
            },
        }
    }
}

impl Default for McpExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionConfig {
    #[serde(default)]
    #[allow(dead_code)]
    instructions: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    server_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionRules {
    /// Only a call to this tool counts; any tool otherwise.
    #[serde(default)]
    required_tool: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ToolCalled {
    tool: String,
    #[serde(default)]
    client: Option<String>,
}

#[async_trait]
impl Extension for McpExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    fn validate_config(
        &self,
        step_type: &str,
        config: &Value,
        validation: &Value,
    ) -> Result<(), Vec<String>> {
        if step_type != STEP_TYPE {
            return Err(unsupported_step_type(ID, step_type));
        }
        collect_issues([
            parse_config::<ConnectionConfig>("config", config).map(|_| ()),
            parse_config::<ConnectionRules>("validation", validation).map(|_| ()),
        ])
    }

    fn handle_event(
        &self,
        ctx: &StepContext<'_>,
        event: &ExtensionEvent,
    ) -> Option<ValidationOutcome> {
        if event.kind != TOOL_CALLED || ctx.step_type != STEP_TYPE {
            return None;
        }
        let call: ToolCalled = serde_json::from_value(event.payload.clone()).ok()?;
        let rules: ConnectionRules = parse_config("validation", ctx.validation).ok()?;
        if rules
            .required_tool
            .as_ref()
            .is_some_and(|tool| *tool != call.tool)
        {
            return None;
        }
        let client = call.client.unwrap_or_else(|| "unknown client".into());
        Some(
            ValidationOutcome::passed(format!("{client} connected and called `{}`", call.tool))
                .with_checks(vec![
                    CheckResult::new("authenticated", "Agent authenticated as you", true)
                        .with_detail(client),
                    CheckResult::new("tool", "Agent called a platform tool", true)
                        .with_detail(call.tool),
                ]),
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn completes_only_for_the_required_tool() {
        let extension = McpExtension::new();
        let config = json!({});
        let validation = json!({ "required_tool": "get_current_step" });
        let ctx = StepContext {
            user_id: "u",
            session_id: "s",
            course_id: "c",
            step_id: "connect",
            step_type: STEP_TYPE,
            config: &config,
            validation: &validation,
        };
        let other = ExtensionEvent::new(TOOL_CALLED, json!({ "tool": "get_session_status" }));
        assert!(extension.handle_event(&ctx, &other).is_none());
        let wanted = ExtensionEvent::new(
            TOOL_CALLED,
            json!({ "tool": "get_current_step", "client": "VS Code" }),
        );
        assert!(extension.handle_event(&ctx, &wanted).unwrap().is_passed());
    }
}
