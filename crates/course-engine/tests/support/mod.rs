#![allow(dead_code)]

use std::path::PathBuf;

use lp_course_engine::{Course, CourseEngine, ExtensionRegistry, parse_definition};
use lp_extensions::{ExtensionOptions, builtin_extensions};

pub fn engine() -> CourseEngine {
    let mut registry = ExtensionRegistry::new();
    let options = ExtensionOptions {
        assets_dir: Some(courses_dir().join("assets")),
    };
    for extension in builtin_extensions(&options) {
        registry.register(extension).unwrap();
    }
    CourseEngine::new(registry).unwrap()
}

pub fn compile(engine: &CourseEngine, yaml: &str) -> Course {
    engine
        .compile(parse_definition(yaml).unwrap())
        .unwrap_or_else(|error| panic!("{error}: {:#?}", error.issues))
}

pub fn courses_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../courses")
}
