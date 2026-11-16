use serde_json::Value;
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct StoredVersion {
    pub id: Uuid,
    pub course_id: Uuid,
    pub checksum: String,
    pub definition: Value,
}

pub async fn upsert_course(
    connection: &mut PgConnection,
    slug: &str,
    title: &str,
    category: &str,
    difficulty: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO courses (id, slug, title, category, difficulty, source) VALUES ($1, $2, $3, $4, $5, 'file')
         ON CONFLICT (slug) DO UPDATE
           SET title = EXCLUDED.title, category = EXCLUDED.category,
               difficulty = EXCLUDED.difficulty, source = 'file', updated_at = now()
         RETURNING id",
    )
    .bind(Uuid::now_v7())
    .bind(slug)
    .bind(title)
    .bind(category)
    .bind(difficulty)
    .fetch_one(connection)
    .await
}

pub async fn find_version_by_number(
    connection: &mut PgConnection,
    course_id: Uuid,
    version: &str,
) -> Result<Option<StoredVersion>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, course_id, checksum, definition FROM course_versions WHERE course_id = $1 AND version = $2",
    )
    .bind(course_id)
    .bind(version)
    .fetch_optional(connection)
    .await
}

pub async fn find_version(
    executor: impl PgExecutor<'_>,
    id: Uuid,
) -> Result<Option<StoredVersion>, sqlx::Error> {
    sqlx::query_as("SELECT id, course_id, checksum, definition FROM course_versions WHERE id = $1")
        .bind(id)
        .fetch_optional(executor)
        .await
}

pub async fn insert_version(
    connection: &mut PgConnection,
    course_id: Uuid,
    version: &str,
    checksum: &str,
    definition: &Value,
    authored: Option<(&str, Uuid)>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO course_versions (id, course_id, version, checksum, definition, source_text, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(Uuid::now_v7())
    .bind(course_id)
    .bind(version)
    .bind(checksum)
    .bind(definition)
    .bind(authored.map(|(source, _)| source))
    .bind(authored.map(|(_, user)| user))
    .fetch_one(connection)
    .await
}

pub async fn replace_version_definition(
    connection: &mut PgConnection,
    version_id: Uuid,
    checksum: &str,
    definition: &Value,
    source_text: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE course_versions SET checksum = $2, definition = $3, source_text = $4 WHERE id = $1",
    )
    .bind(version_id)
    .bind(checksum)
    .bind(definition)
    .bind(source_text)
    .execute(connection)
    .await?;
    Ok(())
}

pub async fn set_current_version(
    connection: &mut PgConnection,
    course_id: Uuid,
    version_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE courses SET current_version_id = $2, updated_at = now() WHERE id = $1")
        .bind(course_id)
        .bind(version_id)
        .execute(connection)
        .await?;
    Ok(())
}

/// Returns `None` when the slug belongs to a file-managed course.
pub async fn upsert_authored_course(
    connection: &mut PgConnection,
    slug: &str,
    title: &str,
    category: &str,
    difficulty: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO courses (id, slug, title, category, difficulty, source) VALUES ($1, $2, $3, $4, $5, 'authored')
         ON CONFLICT (slug) DO UPDATE
           SET title = EXCLUDED.title, category = EXCLUDED.category,
               difficulty = EXCLUDED.difficulty, updated_at = now()
           WHERE courses.source = 'authored'
         RETURNING id",
    )
    .bind(Uuid::now_v7())
    .bind(slug)
    .bind(title)
    .bind(category)
    .bind(difficulty)
    .fetch_optional(connection)
    .await
}

pub async fn count_sessions_for_version(
    connection: &mut PgConnection,
    version_id: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM course_sessions WHERE course_version_id = $1")
        .bind(version_id)
        .fetch_one(connection)
        .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct AuthoredCurrent {
    pub course_id: Uuid,
    pub version_id: Uuid,
    pub definition: Value,
}

pub async fn authored_current_versions(
    connection: &mut PgConnection,
) -> Result<Vec<AuthoredCurrent>, sqlx::Error> {
    sqlx::query_as(
        "SELECT c.id AS course_id, v.id AS version_id, v.definition
         FROM courses c JOIN course_versions v ON v.id = c.current_version_id
         WHERE c.source = 'authored'",
    )
    .fetch_all(connection)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct CourseRecord {
    pub slug: String,
    pub source: String,
    pub version: Option<String>,
    pub definition: Option<Value>,
    pub source_text: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub session_count: i64,
    pub completed_count: i64,
}

macro_rules! select_records {
    ($filter:literal) => {
        concat!(
            "SELECT c.slug, c.source, v.version, v.definition, v.source_text, c.updated_at, ",
            "(SELECT count(*) FROM course_sessions s WHERE s.course_id = c.id) AS session_count, ",
            "(SELECT count(*) FROM course_sessions s WHERE s.course_id = c.id AND s.status = 'completed') AS completed_count ",
            "FROM courses c LEFT JOIN course_versions v ON v.id = c.current_version_id ",
            $filter
        )
    };
}

pub async fn list_records(executor: impl PgExecutor<'_>) -> Result<Vec<CourseRecord>, sqlx::Error> {
    sqlx::query_as(select_records!("ORDER BY c.updated_at DESC"))
        .fetch_all(executor)
        .await
}

pub async fn find_record(
    executor: impl PgExecutor<'_>,
    slug: &str,
) -> Result<Option<CourseRecord>, sqlx::Error> {
    sqlx::query_as(select_records!("WHERE c.slug = $1"))
        .bind(slug)
        .fetch_optional(executor)
        .await
}
