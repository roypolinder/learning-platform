# Single image: static SvelteKit build served by the Rust API.
# Builds natively for the target platform, e.g. `docker buildx build --platform linux/arm64`
# for a Raspberry Pi.

FROM node:24-bookworm-slim AS web
WORKDIR /src
RUN corepack enable
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY apps/web/package.json apps/web/
RUN pnpm install --frozen-lockfile --filter web
COPY apps/web apps/web
RUN pnpm --filter web build

FROM rust:1-bookworm AS api
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY apps/api apps/api
# Embedded in the binary as the course authoring guide.
COPY docs/course-authoring.md docs/
COPY courses/schema courses/schema
RUN cargo build --release -p lp-api --bin learning-platform

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /app learning
WORKDIR /app
COPY --from=api /src/target/release/learning-platform /usr/local/bin/learning-platform
COPY --from=web /src/apps/web/build /app/web
COPY courses /app/courses
USER learning
ENV API_BIND_ADDRESS=0.0.0.0:8080 \
    COURSES_DIR=/app/courses \
    WEB_DIST_DIR=/app/web
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD ["/bin/sh", "-c", "exec 3<>/dev/tcp/127.0.0.1/8080"]
ENTRYPOINT ["learning-platform"]
CMD ["serve"]
