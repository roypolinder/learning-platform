use std::collections::HashMap;

use lp_core::ExtensionManifest;
use sqlx::PgPool;
use uuid::Uuid;

/// Records the compiled-in extensions so operators can see what each deployment
/// provides and disable extensions without a rebuild in the future.
pub async fn sync<'a>(
    db: &PgPool,
    manifests: impl Iterator<Item = &'a ExtensionManifest>,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for manifest in manifests {
        sqlx::query(
            "INSERT INTO extension_registrations (id, extension_id, name, version, manifest)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (extension_id) DO UPDATE
               SET name = EXCLUDED.name, version = EXCLUDED.version,
                   manifest = EXCLUDED.manifest, updated_at = now()",
        )
        .bind(Uuid::now_v7())
        .bind(manifest.id)
        .bind(manifest.name)
        .bind(manifest.version)
        .bind(serde_json::to_value(manifest).unwrap_or_default())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

pub async fn enabled_flags(db: &PgPool) -> Result<HashMap<String, bool>, sqlx::Error> {
    let rows: Vec<(String, bool)> =
        sqlx::query_as("SELECT extension_id, enabled FROM extension_registrations")
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().collect())
}
