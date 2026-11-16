ALTER TABLE users
    ADD COLUMN role TEXT NOT NULL DEFAULT 'learner'
    CHECK (role IN ('learner', 'super_admin'));

-- `file` courses are owned by YAML in COURSES_DIR; `authored` ones are edited in the app.
ALTER TABLE courses
    ADD COLUMN source TEXT NOT NULL DEFAULT 'file'
    CHECK (source IN ('file', 'authored'));

-- Original YAML of authored versions, so comments and formatting survive edits.
ALTER TABLE course_versions
    ADD COLUMN source_text TEXT,
    ADD COLUMN created_by UUID REFERENCES users (id) ON DELETE SET NULL;
