mod support;

use lp_course_engine::{
    Audience, Locale, check_renders, load_directory, load_themes, parse_definition,
    render_definition, view,
};

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
    assert_eq!(step.depends_on, vec!["previous-course".to_owned()]);
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

#[test]
fn every_course_renders_for_every_theme_and_language() {
    let engine = support::engine();
    let courses = load_directory(&engine, &support::courses_dir()).unwrap();
    let themes = load_themes(&support::courses_dir().join("themes")).unwrap();
    assert!(themes.selectable().len() >= 10);
    let errors = check_renders(&engine, &courses, &themes);
    assert!(
        errors.is_empty(),
        "{}",
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn rendering_fills_in_the_theme_and_the_language() {
    let engine = support::engine();
    let courses = load_directory(&engine, &support::courses_dir()).unwrap();
    let themes = load_themes(&support::courses_dir().join("themes")).unwrap();
    let (_, course) = courses
        .iter()
        .find(|(_, course)| course.id == "semantic-matching")
        .unwrap();
    let bikes = themes.get("bike-rental").unwrap();
    let dutch = engine
        .render(
            course,
            Audience {
                locale: Locale::Nl,
                theme: bikes,
            },
        )
        .unwrap();
    assert_eq!(dutch.title, "Semantic search");
    assert!(dutch.description.contains("Velora Bikes"));
    assert!(dutch.description.contains("zoeken"));
    let search = dutch.step("search").unwrap();
    let text = search.validation.to_string();
    assert!(text.contains("fiets voor moeder"), "{text}");
    assert!(text.contains("city-bike-child-seat"));
    assert!(!text.contains("{{theme."));

    // Before a project is chosen, the neutral theme is used and nothing is left unfilled.
    let neutral = engine
        .render(
            course,
            Audience {
                locale: Locale::En,
                theme: themes.neutral(),
            },
        )
        .unwrap();
    assert!(
        !serde_json::to_string(&neutral.definition)
            .unwrap()
            .contains("{{theme.")
    );
}

#[test]
fn translation_files_only_change_text() {
    let engine = support::engine();
    let courses = load_directory(&engine, &support::courses_dir()).unwrap();
    for (path, course) in &courses {
        assert!(
            course.definition.translations.contains_key(&Locale::Nl),
            "{} has no Dutch translation",
            path.display()
        );
        let english = engine
            .render(
                course,
                Audience {
                    locale: Locale::En,
                    theme: load_themes(&support::courses_dir().join("themes"))
                        .unwrap()
                        .neutral(),
                },
            )
            .unwrap();
        let steps_en: Vec<_> = english
            .steps
            .iter()
            .map(|s| (&s.id, &s.step_type, &s.depends_on))
            .collect();
        let themes = load_themes(&support::courses_dir().join("themes")).unwrap();
        let dutch = engine
            .render(
                course,
                Audience {
                    locale: Locale::Nl,
                    theme: themes.neutral(),
                },
            )
            .unwrap();
        let steps_nl: Vec<_> = dutch
            .steps
            .iter()
            .map(|s| (&s.id, &s.step_type, &s.depends_on))
            .collect();
        assert_eq!(steps_en, steps_nl, "{}", path.display());
    }
}
