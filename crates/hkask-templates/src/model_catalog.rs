//! Model Catalog Seeder

use crate::okapi_config::OkapiConfig;
use hkask_storage::ModelRegistryStore;

/// Seed the model registry with initial catalog
pub fn seed_model_catalog(
    _registry: &ModelRegistryStore,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
