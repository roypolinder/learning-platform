use lp_core::ExtensionError;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub fn parse_submission<T: DeserializeOwned>(payload: &Value) -> Result<T, ExtensionError> {
    serde_json::from_value(payload.clone())
        .map_err(|error| ExtensionError::InvalidSubmission(error.to_string()))
}

pub fn collect_issues(
    results: impl IntoIterator<Item = Result<(), Vec<String>>>,
) -> Result<(), Vec<String>> {
    let issues: Vec<String> = results
        .into_iter()
        .filter_map(Result::err)
        .flatten()
        .collect();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

pub fn unsupported_step_type(extension: &str, step_type: &str) -> Vec<String> {
    vec![format!(
        "extension `{extension}` cannot handle `{step_type}` steps"
    )]
}
