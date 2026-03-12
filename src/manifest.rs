use serde::Serialize;

use crate::metadata_types;

#[derive(Serialize)]
pub struct Manifest {
    pub version: String,
    pub supported_metadata: Vec<MetadataEntry>,
}

#[derive(Serialize, Clone)]
pub struct MetadataEntry {
    pub extension: String,
    #[serde(rename = "type")]
    pub meta_type: String,
    pub category: String,
    pub order_sensitive: bool,
    pub supported_formats: Vec<String>,
}

pub fn build_manifest() -> Manifest {
    let entries = metadata_types::METADATA_TYPES
        .iter()
        .map(|t| {
            let (meta_type, category, order_sensitive) = metadata_types::classify(t.extension);
            MetadataEntry {
                extension: t.extension.to_string(),
                meta_type: meta_type.to_string(),
                category: category.to_string(),
                order_sensitive,
                supported_formats: vec!["yaml".to_string(), "json".to_string()],
            }
        })
        .collect();

    Manifest {
        version: env!("CARGO_PKG_VERSION").to_string(),
        supported_metadata: entries,
    }
}
