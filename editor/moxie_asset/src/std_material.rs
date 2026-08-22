use std::any::TypeId;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::ecs::error::{BevyError, Severity};
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::pbr::StandardMaterial;
use bevy::reflect::serde::{
    TypedReflectDeserializer, TypedReflectSerializer,
};
use bevy::reflect::{
    PartialReflect, ReflectFromReflect, TypePath, TypeRegistry,
};
use serde::de::DeserializeSeed;

#[derive(Default, TypePath)]
pub struct StdMaterialAssetLoader {
    registry: AppTypeRegistry,
}

impl StdMaterialAssetLoader {
    pub fn new(registry: &AppTypeRegistry) -> Self {
        Self {
            registry: registry.clone(),
        }
    }
}

impl AssetLoader for StdMaterialAssetLoader {
    type Asset = StandardMaterial;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let type_registry = self.registry.read();
        material_from_ron(&bytes, &type_registry)
    }

    fn extensions(&self) -> &[&str] {
        &["mat"]
    }
}

fn material_from_ron(
    bytes: &[u8],
    type_registry: &TypeRegistry,
) -> Result<StandardMaterial, BevyError> {
    let registration = type_registry
        .get(TypeId::of::<StandardMaterial>())
        .ok_or_else(|| {
            BevyError::new(
                Severity::Error,
                "StandardMaterial is not registered".to_string(),
            )
        })?;

    let mut ron =
        ron::de::Deserializer::from_bytes(bytes).map_err(|e| {
            BevyError::new(Severity::Error, e.to_string())
        })?;

    let deserializer =
        TypedReflectDeserializer::new(registration, type_registry);
    let dynamic: Box<dyn PartialReflect> =
        deserializer.deserialize(&mut ron).map_err(|e| {
            BevyError::new(Severity::Error, e.to_string())
        })?;

    let reflect_from_reflect = type_registry
        .get_type_data::<ReflectFromReflect>(
            dynamic
                .get_represented_type_info()
                .ok_or_else(|| {
                    BevyError::new(
                        Severity::Error,
                        "deserialized value has no type info"
                            .to_string(),
                    )
                })?
                .type_id(),
        )
        .ok_or_else(|| {
            BevyError::new(
                Severity::Error,
                "StandardMaterial has no ReflectFromReflect data"
                    .to_string(),
            )
        })?;

    let reflected = reflect_from_reflect
        .from_reflect(dynamic.as_partial_reflect())
        .ok_or_else(|| {
            BevyError::new(
                Severity::Error,
                "failed to convert reflected value".to_string(),
            )
        })?;

    Ok(*reflected.downcast::<StandardMaterial>().map_err(|_| {
        BevyError::new(
            Severity::Error,
            "deserialized value is not a StandardMaterial"
                .to_string(),
        )
    })?)
}

/// Serializes `material` through reflection into a pretty-printed RON
/// string, which a `.mat` loader can read back.
pub fn serialize_to_ron(
    material: &StandardMaterial,
    registry: &TypeRegistry,
) -> Result<String, BevyError> {
    let serializer = TypedReflectSerializer::new(
        material.as_partial_reflect(),
        registry,
    );
    ron::ser::to_string_pretty(
        &serializer,
        ron::ser::PrettyConfig::default()
            .indentor("  ".to_string())
            .new_line("\n".to_string()),
    )
    .map_err(|e| {
        BevyError::new(
            Severity::Error,
            format!("material serialization failed: {e}"),
        )
    })
}
