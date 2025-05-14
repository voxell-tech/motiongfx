use bevy::prelude::*;

use crate::ease::{cubic, EaseFn};
use crate::f32lerp::F32Lerp;
use crate::sequence::{MultiSeqOrd, Sequence};
use crate::ThreadSafe;

/// Function for interpolating a type based on a [`f32`] time.
pub type InterpFn<T> = fn(start: &T, end: &T, t: f32) -> T;
/// Function for getting a mutable reference of a field of type `U` in type `T`.
/// Type `U` can be similar to `T` as well.
pub type GetFieldMut<T, F> = fn(source: &mut T) -> &mut F;
/// Function for getting the end value of an action based on the current (start) value.
/// The value can be a field of type `F` in type `T`.
/// Type `F` can be similar to `T` as well.
///
/// # Example
/// ```
/// use motiongfx_core::action::ActionFn;
///
/// struct Point {
///     pub x: f32,
///     pub y: f32,
/// }
///
/// let point = Point {
///     x: 1.0,
///     y: 2.0,
/// }
/// let move_pointx_action = |point: &Point| { point.x += 10.0 };
/// ```
pub type ActionFn<T, F> = fn(source: &T) -> &F;
// TODO: Move the example to [`Action`]

/// Creates an [`Action`] and changes the animated value to the end value.
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
/// use motiongfx_core::prelude::*;
///
/// let mut world = World::new();
/// let mut transform = Transform::default();
/// let id = world.spawn(transform).id();
///
/// // Creates an action on `translation.x`
/// // of a `Transform` component
/// let action = act!(
///     (id, Transform),
///     start = { transform }.translation.x,
///     end = transform.translation.x + 1.0,
/// );
/// ```
#[macro_export]
macro_rules! act {
    (
        ($target_id:expr, $comp_ty:ty),
        start = { $root:expr }.$($path:tt).+,
        end = $value:expr,
    ) => {
        {
            let action = $crate::action::Action::new_f32lerp(
                $target_id,
                $root.$($path).+.clone(),
                $value.clone(),
                |source: &mut $comp_ty| &mut source.$($path).+,
            );

            $root.$($path).+ = $value;

            action
        }
    };
    (
        ($target_id:expr, $comp_ty:ty),
        start = { $root:expr },
        end = $value:expr,
    ) => {
        {
            let action = $crate::action::Action::new_f32lerp(
                $target_id,
                $root.clone(),
                $value.clone(),
                |source: &mut $comp_ty| source,
            );

            #[allow(unused_assignments)]
            {
                $root = $value;
            }

            action
        }
    };
    (
        ($target_id:expr, $comp_ty:ty),
        start = { $root:expr }.$($path:tt).+,
        end = $value:expr,
        interp = $interp:expr,
    ) => {
        {
            let action = $crate::action::Action::new(
                $target_id,
                $root.$($path).+.clone(),
                $value.clone(),
                |source: &mut $comp_ty| &mut source.$($path).+,
                $interp,
            );

            $root.$($path).+ = $value;

            action
        }
    };
    (
        ($target_id:expr, $comp_ty:ty),
        start = { $root:expr },
        end = $value:expr,
        interp = $interp:expr,
    ) => {
        {
            let action = $crate::action::Action::new_f32lerp(
                $target_id,
                $root.clone(),
                $value.clone(),
                |source: &mut $comp_ty| source,
                $interp,
            );

            #[allow(unused_assignments)]
            {
                $root = $value;
            }

            action
        }
    };
}

pub use act;

/// Basic data structure to describe an animation action.
#[derive(Component, Clone, Copy)]
pub struct Action<T, F> {
    /// Target [`Entity`] for [`Component`] manipulation.
    pub(crate) entity: Entity,
    /// Initial value of the action.
    pub(crate) start: F,
    /// Final value of the action.
    pub(crate) end: F,
    /// Function for getting a mutable reference of a field (or itself) from the component.
    pub(crate) get_field_fn: GetFieldMut<T, F>,
    /// Function for interpolating the field value based on a [`f32`] time.
    pub(crate) interp_fn: InterpFn<F>,
    /// Function for easing the [`f32`] time value for the action.
    pub(crate) ease_fn: EaseFn,
}

impl<T, F> Action<T, F> {
    /// Creates a new [`Action`].
    pub fn new(
        entity: Entity,
        start: F,
        end: F,
        interp_fn: InterpFn<F>,
        get_field_fn: GetFieldMut<T, F>,
    ) -> Self {
        Self {
            entity,
            start,
            end,
            get_field_fn,
            interp_fn,
            ease_fn: cubic::ease_in_out,
        }
    }

    /// Overwrite the existing [easing function](EaseFn).
    pub fn with_ease(mut self, ease_fn: EaseFn) -> Self {
        self.ease_fn = ease_fn;
        self
    }

    /// Overwrite the existing [interpolation function](InterpFn).
    pub fn with_interp(mut self, interp_fn: InterpFn<F>) -> Self {
        self.interp_fn = interp_fn;
        self
    }

    /// Convert an [`Action`] into a [`Motion`] by adding a duration.
    pub fn animate(self, duration: f32) -> Motion<T, F> {
        Motion {
            action: self,
            duration,
        }
    }
}

impl<T, F> Action<T, F>
where
    F: F32Lerp,
{
    /// Creates a new [`Action`] with [`F32Lerp`] as the default
    /// [interpolation function](InterpFn).
    pub fn new_f32lerp(
        entity: Entity,
        start: F,
        end: F,
        get_field_fn: GetFieldMut<T, F>,
    ) -> Self {
        Self {
            entity,
            start,
            end,
            get_field_fn,
            interp_fn: F::f32lerp,
            ease_fn: cubic::ease_in_out,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ActionSpan {
    /// Target [`Entity`] for [`Action`].
    action_id: Entity,
    /// Time at which animation should begin.
    pub(crate) start_time: f32,
    /// Duration of animation in seconds.
    pub(crate) duration: f32,
    /// Slide that this action belongs to.
    pub(crate) slide_index: usize,
}

impl ActionSpan {
    pub fn new(action_id: Entity) -> Self {
        Self {
            action_id,
            start_time: 0.0,
            duration: 0.0,
            slide_index: 0,
        }
    }

    pub fn id(&self) -> Entity {
        self.action_id
    }

    #[inline]
    pub fn with_start_time(mut self, start_time: f32) -> Self {
        self.start_time = start_time;
        self
    }

    #[inline]
    pub fn end_time(&self) -> f32 {
        self.start_time + self.duration
    }
}

#[derive(Clone, Copy)]
pub struct Motion<T, U> {
    pub action: Action<T, U>,
    pub duration: f32,
}

pub struct SequenceBuilder<'w, 's> {
    commands: Commands<'w, 's>,
    sequences: Vec<Sequence>,
}

impl<'a> SequenceBuilder<'a, 'a> {
    /// Converts a [`Motion`] into a [`SequenceBuilder`].
    pub fn add_motion<T, U>(mut self, motion: Motion<T, U>) -> Self
    where
        T: ThreadSafe,
        U: ThreadSafe,
    {
        self.sequences.push(self.commands.play_motion(motion));
        self
    }

    pub fn build(self) -> Vec<Sequence> {
        self.sequences
    }
}

impl MultiSeqOrd for SequenceBuilder<'_, '_> {
    fn chain(self) -> Sequence {
        self.sequences.chain()
    }

    fn all(self) -> Sequence {
        self.sequences.all()
    }

    fn any(self) -> Sequence {
        self.sequences.any()
    }

    fn flow(self, delay: f32) -> Sequence {
        self.sequences.flow(delay)
    }
}

pub trait SequenceBuilderExt<'w> {
    /// Converts a [`Motion`] into a [`Sequence`].
    fn play_motion<T, U>(&mut self, motion: Motion<T, U>) -> Sequence
    where
        T: ThreadSafe,
        U: ThreadSafe;

    /// Converts a [`Motion`] into a [`SequenceBuilder`].
    fn add_motion<T, U>(
        &mut self,
        motion: Motion<T, U>,
    ) -> SequenceBuilder<'w, '_>
    where
        T: ThreadSafe,
        U: ThreadSafe;

    fn sleep(&mut self, duration: f32) -> Sequence;
}

impl<'w> SequenceBuilderExt<'w> for Commands<'w, '_> {
    fn play_motion<T, U>(&mut self, motion: Motion<T, U>) -> Sequence
    where
        T: ThreadSafe,
        U: ThreadSafe,
    {
        let action_id = self.spawn(motion.action).id();
        let mut span = ActionSpan::new(action_id);
        span.duration = motion.duration;

        Sequence::single(span)
    }

    fn add_motion<T, U>(
        &mut self,
        motion: Motion<T, U>,
    ) -> SequenceBuilder<'w, '_>
    where
        T: ThreadSafe,
        U: ThreadSafe,
    {
        let mut commands = self.reborrow();
        let sequences = vec![commands.play_motion(motion)];
        SequenceBuilder {
            commands,
            sequences,
        }
    }

    fn sleep(&mut self, duration: f32) -> Sequence {
        Sequence::empty(duration)
    }
}
