use bevy::reflect::TypePath;
use bevy_asset::{
    Asset, AssetIndex, Assets, DirectAssetAccessExt, UntypedAssetId,
};
use bevy_ecs::{
    bundle::Bundle, component::Component, entity::Entity,
};
use bevy_log::tracing_subscriber;
use bevy_motiongfx::{prelude::SubjectSource, world::*};
use rstest::*;
use std::{
    any::TypeId,
    sync::{Arc, Mutex},
};
use tracing::subscriber::with_default;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct BufWriter(Arc<Mutex<Vec<u8>>>);

impl BufWriter {
    fn get_output(&self) -> String {
        let buf = self.0.lock().unwrap();
        String::from_utf8_lossy(&buf).into_owned()
    }
}

impl<'a> MakeWriter<'a> for BufWriter {
    type Writer = BufWriterGuard<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        BufWriterGuard(self.0.lock().unwrap())
    }
}

struct BufWriterGuard<'a>(std::sync::MutexGuard<'a, Vec<u8>>);

impl std::io::Write for BufWriterGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

#[derive(Component)]
struct DummyComponent;

#[derive(Asset, TypePath)]
struct DummyAsset;

fn subscriber() -> (impl tracing::Subscriber, BufWriter) {
    let buf = BufWriter::default();
    (
        tracing_subscriber::fmt::SubscriberBuilder::default()
            .with_writer(buf.clone())
            .finish(),
        buf,
    )
}

#[rstest]
#[case::has_component(DummyComponent, "")]
#[case::no_component((), "Entity 0v0 does not have component logging::DummyComponent")]
fn logs_component_presence<T: Bundle>(
    #[case] bundle: T,
    #[case] log: &str,
) {
    // make bevy world
    let mut world = bevy_ecs::world::World::new();

    // insert entity with without component
    let entity = world.spawn(bundle).id();

    let bevy_world = BevyWorld::from_ref(&world);

    let (subscriber, buffer) = subscriber();
    // run function in tracing block
    with_default(subscriber, || {
        let _ = <bevy_motiongfx::world::BevyWorld as SubjectSource<
            Entity,
            DummyComponent,
        >>::get_source(bevy_world, entity);
    });
    let out = buffer.get_output();
    // println!("{}", out);
    assert!(out.contains(log))
}

#[rstest]
#[case::has_asset(true, true, "")]
#[case::no_resource(
    false,
    false,
    "Asset type \"bevy_asset::assets::Assets<logging::DummyAsset>\" has not been loaded yet as a resource"
)]
#[case::no_asset_has_log(
    true,
    false,
    "Asset collection bevy_asset::assets::Assets<logging::DummyAsset> does not have asset_id AssetId<logging::DummyAsset>{ index: 0, generation: 0}"
)]
fn logs_asset_presence(
    #[case] has_resource: bool,
    #[case] has_asset: bool,
    #[case] log: &str,
) {
    // make bevy world
    let mut world = bevy_ecs::world::World::new();
    if has_resource {
        world.init_resource::<Assets<DummyAsset>>();
    }

    // insert entity with without component
    let entity = if has_asset {
        world.add_asset::<DummyAsset>(DummyAsset).untyped().id()
    } else {
        UntypedAssetId::Index {
            type_id: TypeId::of::<DummyAsset>(),
            index: AssetIndex::from_bits(0),
        }
    };

    let bevy_world = BevyWorld::from_ref(&world);

    let (subscriber, buffer) = subscriber();
    // run function in tracing block
    with_default(subscriber, || {
        let _ = <bevy_motiongfx::world::BevyWorld as SubjectSource<
            UntypedAssetId,
            DummyAsset,
        >>::get_source(bevy_world, entity);
    });
    let out = buffer.get_output();
    // println!("{}", out);
    assert!(out.contains(log))
}
