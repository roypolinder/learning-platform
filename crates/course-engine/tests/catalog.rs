mod support;

use lp_course_engine::{load_directory, parse_definition, render_definition, view};

#[test]
fn example_courses_compile() {
    let engine = support::engine();
    let courses = load_directory(&engine, &support::courses_dir()).unwrap_or_else(|errors| {
        panic!(
            "{}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    let ids: Vec<&str> = courses
        .iter()
        .map(|(_, course)| course.id.as_str())
        .collect();
    assert!(ids.contains(&"mcp-first-connection"));
    assert!(ids.contains(&"agent-skill-basics"));
}

#[test]
fn rendered_definitions_round_trip() {
    let engine = support::engine();
    let courses = load_directory(&engine, &support::courses_dir()).unwrap();
    for (path, course) in courses {
        let stored = serde_json::to_value(&course.definition).unwrap();
        let yaml = render_definition(&stored).unwrap();
        let reparsed = parse_definition(&yaml)
            .unwrap_or_else(|error| panic!("{}: {error}\n{yaml}", path.display()));
        assert_eq!(reparsed, course.definition, "{}", path.display());
    }
}

#[test]
fn example_courses_match_json_schema() {
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(support::courses_dir().join("schema/course.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for entry in std::fs::read_dir(support::courses_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let document: serde_json::Value = serde_saphyr::from_str(&source).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&document)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{}: {errors:#?}", path.display());
    }
}

#[test]
fn reusable_sections_are_inlined_with_prefixed_ids() {
    let engine = support::engine();
    let source =
        std::fs::read_to_string(support::courses_dir().join("agent-skill-basics.yaml")).unwrap();
    let course = engine.compile(parse_definition(&source).unwrap()).unwrap();
    let step = course.step("connection.reconnect").expect("inlined step");
    assert_eq!(step.step_type, "mcp_connection_check");
    assert_eq!(step.depends_on, vec!["good-description".to_owned()]);
    assert_eq!(
        course.step("write-skill").unwrap().depends_on,
        vec!["connection.reconnect".to_owned()]
    );
}

#[test]
fn public_views_never_leak_private_rules_or_answers() {
    let engine = support::engine();
    let source =
        std::fs::read_to_string(support::courses_dir().join("mcp-first-connection.yaml")).unwrap();
    let course = engine.compile(parse_definition(&source).unwrap()).unwrap();
    let detail = serde_json::to_string(&view::course_detail(&engine, &course)).unwrap();
    assert!(!detail.contains("\"correct\""));
    assert!(!detail.contains("required_tool"));
    assert!(!detail.contains("Remote servers use Streamable HTTP; stdio only works"));
}
