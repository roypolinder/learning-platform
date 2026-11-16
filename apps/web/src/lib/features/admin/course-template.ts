/** Starting point for a new course in the studio. */
export const COURSE_TEMPLATE = `id: my-new-course            # stable slug: lowercase, never change after publishing
version: 1.0.0               # bump once learners have started a version
title: My new course
summary: One sentence shown in the catalog.
description: |
  Markdown shown on the course page. Explain what learners will build.
category: General
difficulty: beginner         # beginner | intermediate | advanced
estimated_minutes: 15
tags: []
objectives:
  - Describe what the learner can do afterwards

sections:
  - id: basics
    title: Basics
    steps:
      - id: intro
        type: explanation
        title: Introduction
        estimated_minutes: 3
        body: |
          Markdown content for the learner.

      - id: check
        type: quiz
        title: Check your understanding
        config:
          pass_score: 0.6
          questions:
            - id: q1
              prompt: Which answer is correct?
              options:
                - { id: a, text: This one, correct: true }
                - { id: b, text: Not this one }
`;
