-- Interface and course language. Course content falls back to English per field.
ALTER TABLE users
    ADD COLUMN locale TEXT NOT NULL DEFAULT 'en' CHECK (locale IN ('en', 'nl'));

-- The website a learner builds throughout the course track. Chosen once, never changed:
-- later courses build on what earlier ones produced for this theme.
CREATE TABLE learner_projects (
    user_id UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    theme_id TEXT NOT NULL,
    stack TEXT NOT NULL CHECK (stack IN ('javascript', 'typescript', 'python', 'other')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Personal notes, kept per course in the glossary sidebar.
CREATE TABLE course_notes (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    course_slug TEXT NOT NULL,
    body TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 5000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX course_notes_user_course ON course_notes (user_id, course_slug, created_at);
