use std::net::SocketAddr;
use std::process::ExitCode;

use lp_api::app::{self, build_engine};
use lp_api::config::AppConfig;
use lp_api::db;
use lp_course_engine::{CatalogError, check_renders, load_directory, load_themes};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

const USAGE: &str =
    "usage: learning-platform [serve | migrate | check-courses | grant-super-admin <email>]";

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,lp_api=debug,sqlx=warn,tower_http=info")),
        )
        .init();

    let command = std::env::args().nth(1).unwrap_or_else(|| "serve".into());
    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("configuration error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let result = match command.as_str() {
        "serve" => serve(config).await,
        "migrate" => migrate(config).await,
        "check-courses" => check_courses(&config),
        "grant-super-admin" => match std::env::args().nth(2) {
            Some(email) => grant_super_admin(config, &email).await,
            None => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        },
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn serve(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let address = config.bind_address;
    let state = app::bootstrap(config).await?;
    let shutdown = CancellationToken::new();
    let router = app::router(state, shutdown.clone());
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!("listening on http://{address} (MCP endpoint: /mcp)");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown_signal().await;
        tracing::info!("shutting down");
        shutdown.cancel();
    })
    .await?;
    Ok(())
}

/// Ctrl+C in a terminal, SIGTERM from systemd or Docker.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = terminate.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn migrate(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let pool = db::connect(&config.database_url, 2).await?;
    db::migrate(&pool).await?;
    tracing::info!("migrations applied");
    Ok(())
}

async fn grant_super_admin(
    config: AppConfig,
    email: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = db::connect(&config.database_url, 2).await?;
    db::migrate(&pool).await?;
    let email = email.trim().to_lowercase();
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE lower(email) = $1)")
            .bind(&email)
            .fetch_one(&pool)
            .await?;
    if !exists {
        return Err(format!("no account with email {email}; register first").into());
    }
    lp_api::domains::users::repository::promote_super_admins(&pool, std::slice::from_ref(&email))
        .await?;
    println!("{email} is now a super admin");
    Ok(())
}

fn check_courses(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let engine = build_engine(config)?;
    let report = |errors: &[CatalogError]| {
        for error in errors {
            eprintln!("error  {error}");
            if let CatalogError::Invalid { source, .. } = error {
                for issue in &source.issues {
                    eprintln!("       - {issue}");
                }
            }
        }
    };
    let themes = load_themes(&config.courses_dir.join("themes")).map_err(|errors| {
        report(&errors);
        format!("{} theme problem(s)", errors.len())
    })?;
    let courses = load_directory(&engine, &config.courses_dir).map_err(|errors| {
        report(&errors);
        format!("{} course file(s) invalid", errors.len())
    })?;
    let render_errors = check_renders(&engine, &courses, &themes);
    if !render_errors.is_empty() {
        report(&render_errors);
        return Err(format!(
            "{} course(s) do not render for every theme",
            render_errors.len()
        )
        .into());
    }
    for (path, course) in &courses {
        println!(
            "ok  {} v{} ({} steps, {} languages) — {}",
            course.id,
            course.version,
            course.steps.len(),
            course.definition.translations.len() + 1,
            path.display()
        );
    }
    println!(
        "ok  {} themes, every course renders in every language and theme",
        themes.selectable().len()
    );
    Ok(())
}
