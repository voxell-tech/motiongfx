use bevy::prelude::*;
use motiongfx_engine::prelude::*;

pub struct MotionGfxCommonPlugin;

impl Plugin for MotionGfxCommonPlugin {
    fn build(&self, app: &mut App) {
        app.animate_component(field_bundle!(<Transform>))
            .animate_component(field_bundle!(
                <Transform>::translation
            ))
            .animate_component(field_bundle!(<Transform>::scale))
            .animate_component(field_bundle!(<Transform>::rotation))
            .animate_component(field_bundle!(
                <Transform>::translation::x
            ))
            .animate_component(field_bundle!(
                <Transform>::translation::y
            ))
            .animate_component(field_bundle!(
                <Transform>::translation::z
            ))
            .animate_component(field_bundle!(<Transform>::scale::x))
            .animate_component(field_bundle!(<Transform>::scale::y))
            .animate_component(field_bundle!(<Transform>::scale::z));

        #[cfg(feature = "bevy_sprite")]
        app.animate_component(field_bundle!(<Sprite>::color))
            .animate_asset::<MeshMaterial2d<_>, _>(field_bundle!(
            <ColorMaterial>::color
        ));

        #[cfg(feature = "bevy_pbr")]
        app.animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::base_color
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::emissive
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::perceptual_roughness
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::metallic
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::reflectance
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::specular_tint
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::diffuse_transmission
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::specular_transmission
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::thickness
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::ior
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(field_bundle!(
            <StandardMaterial>::attenuation_distance
        ))
        .animate_asset::<MeshMaterial3d<_>, _>(
            field_bundle!(<StandardMaterial>::attenuation_color),
        );
    }
}
