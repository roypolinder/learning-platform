use axum::http::HeaderMap;
use axum::http::header::ACCEPT_LANGUAGE;
use lp_course_engine::{Audience, EngineContext, Locale, ThemeCatalog};
use serde::{Deserialize, Serialize};

use crate::http::auth::CurrentUser;

/// The language a course or theme is shown in and the learner's project, if any.
#[derive(Debug, Clone, Default)]
pub struct LearnerProfile {
    pub locale: Locale,
    pub project: Option<Project>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub theme_id: String,
    pub stack: Stack,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// The language a learner builds in. JavaScript is recommended: every later course has
/// a ready path for it, but the checks only look at behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stack {
    Javascript,
    Typescript,
    Python,
    Other,
}

impl Stack {
    pub fn as_str(self) -> &'static str {
        match self {
            Stack::Javascript => "javascript",
            Stack::Typescript => "typescript",
            Stack::Python => "python",
            Stack::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(value.to_owned())).ok()
    }
}

impl LearnerProfile {
    /// Visitors who are not signed in see courses in their browser's language.
    pub fn anonymous(headers: &HeaderMap) -> Self {
        Self {
            locale: request_locale(None, headers),
            project: None,
        }
    }

    pub fn audience<'a>(&self, themes: &'a ThemeCatalog) -> Audience<'a> {
        Audience {
            locale: self.locale,
            theme: themes.resolve(self.project.as_ref().map(|p| p.theme_id.as_str())),
        }
    }

    pub fn has_project(&self) -> bool {
        self.project.is_some()
    }

    pub fn apply_to(&self, context: &mut EngineContext) {
        context.has_project = self.has_project();
    }
}

/// The signed-in user's language, otherwise the best match for `Accept-Language`.
pub fn request_locale(user: Option<&CurrentUser>, headers: &HeaderMap) -> Locale {
    user.and_then(|user| user.locale.parse().ok())
        .or_else(|| {
            headers
                .get(ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok())
                .map(Locale::negotiate)
        })
        .unwrap_or_default()
}
