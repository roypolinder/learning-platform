use std::collections::HashMap;

use lp_course_engine::view::{CourseDetailView, CourseSummaryView};
use serde::{Deserialize, Serialize};

use crate::domains::progress::service::CourseProgress;
use crate::domains::sessions::dto::SessionDto;
use crate::http::pagination::Page;

#[derive(Debug, Default, Deserialize)]
pub struct CourseListQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub difficulty: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct CourseListResponse {
    #[serde(flatten)]
    pub page: Page<CourseSummaryView>,
    pub categories: Vec<String>,
    /// The caller's progress keyed by course id; empty when signed out.
    pub progress: HashMap<String, CourseProgress>,
}

#[derive(Debug, Serialize)]
pub struct CourseDetailResponse {
    pub course: CourseDetailView,
    /// The caller's most relevant session for this course, if signed in.
    pub session: Option<SessionDto>,
}
