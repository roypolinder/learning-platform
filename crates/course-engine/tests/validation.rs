mod support;

use lp_course_engine::parse_definition;

const HEADER: &str = "id: broken\nversion: 1.0.0\ntitle: T\nsummary: S\ndescription: D\ncategory: C\ndifficulty: beginner\nestimated_minutes: 5\n";

fn issues(body: &str) -> Vec<String> {
    let engine = support::engine();
    let definition = parse_definition(&format!("{HEADER}{body}")).expect("parses");
    match engine.compile(definition) {
        Ok(_) => Vec::new(),
        Err(error) => error.issues.iter().map(ToString::to_string).collect(),
    }
}

fn assert_issue(body: &str, needle: &str) {
    let found = issues(body);
    assert!(
        found.iter().any(|issue| issue.contains(needle)),
        "expected `{needle}` in {found:#?}"
    );
}

#[test]
fn rejects_unknown_step_types() {
    assert_issue(
        "sections:\n  - id: a\n    title: A\n    steps:\n      - { id: s, type: teleport, title: S }\n",
        "unknown step type `teleport`",
    );
}

#[test]
fn rejects_unknown_fields() {
    let error = parse_definition(&format!("{HEADER}colour: red\nsections: []\n")).unwrap_err();
    assert!(error.contains("colour"), "{error}");
}

#[test]
fn rejects_dependency_cycles_and_unknown_dependencies() {
    let body = "sections:\n  - id: a\n    title: A\n    steps:\n      - { id: one, type: manual_completion, title: One, depends_on: [two] }\n      - { id: two, type: manual_completion, title: Two, depends_on: [one] }\n      - { id: three, type: manual_completion, title: Three, depends_on: [ghost] }\n";
    assert_issue(body, "dependency cycle");
    assert_issue(body, "unknown step `ghost`");
}

#[test]
fn rejects_duplicate_ids_and_bad_versions() {
    let body = "sections:\n  - id: a\n    title: A\n    steps:\n      - { id: one, type: manual_completion, title: One }\n      - { id: one, type: manual_completion, title: Again }\n";
    assert_issue(body, "duplicate step id");
    let definition =
        parse_definition(&format!("{}{}", HEADER.replace("1.0.0", "v1"), body)).unwrap();
    let error = support::engine().compile(definition).unwrap_err();
    assert!(error.issues.iter().any(|issue| issue.path == "version"));
}

#[test]
fn delegates_config_validation_to_extensions() {
    let body = "sections:\n  - id: a\n    title: A\n    steps:\n      - id: q\n        type: multiple_choice\n        title: Q\n        config:\n          prompt: Pick\n          options: [ { id: a, text: A }, { id: b, text: B } ]\n";
    assert_issue(body, "exactly one correct option");
}

#[test]
fn generic_practical_steps_need_an_extension_or_manual_completion() {
    let body = "sections:\n  - id: a\n    title: A\n    steps:\n      - { id: task, type: external_task, title: Do it }\n";
    assert_issue(body, "need an `extension` or `completion: manual`");
    let manual = "sections:\n  - id: a\n    title: A\n    steps:\n      - { id: task, type: external_task, title: Do it, completion: manual }\n";
    assert!(issues(manual).is_empty());
}

#[test]
fn extension_types_cannot_be_reassigned() {
    let body = "sections:\n  - id: a\n    title: A\n    steps:\n      - { id: c, type: mcp_connection_check, title: C, extension: http }\n";
    assert_issue(body, "handled by `mcp`");
}

#[test]
fn declared_extensions_must_cover_usage() {
    let body = "extensions: [quiz]\nsections:\n  - id: a\n    title: A\n    steps:\n      - { id: c, type: mcp_connection_check, title: C }\n";
    assert_issue(body, "steps use `mcp` but it is not declared");
}
