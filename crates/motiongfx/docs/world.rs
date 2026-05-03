pub use motiongfx::prelude::*;

pub struct World(pub Vec<f32>);

impl SubjectSource<usize, f32> for World {
    fn get_source(&self, id: usize) -> Option<&f32> {
        self.0.get(id)
    }

    fn apply_source<R>(
        &mut self,
        id: usize,
        f: impl FnOnce(&mut f32) -> R,
    ) -> Option<R> {
        self.0.get_mut(id).map(f)
    }
}

pub fn timeline() -> (Registry, Timeline<World>) {
    let mut registry = Registry::new();
    let mut b = registry.create_builder::<World>();
    let track = b
        .act_builder(0usize, path!(<f32>), |x| x + 10.0)
        .with_interp(|a: &f32, b: &f32, t| a + (b - a) * t)
        .play(1.0)
        .compile();
    b.add_tracks(track);
    let timeline = b.compile();
    (registry, timeline)
}
