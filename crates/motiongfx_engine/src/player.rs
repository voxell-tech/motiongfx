use bevy::prelude::*;
use smallvec::SmallVec;

use crate::sequence::{Sequence, SequenceController};
use crate::MotionGfxSet;

pub(super) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_target_time.in_set(MotionGfxSet::TargetTime),
        )
        .add_observer(jump_sequence);
    }
}

pub trait BuildPlayerAppExt {
    fn create_sequence_player(
        &mut self,
        sequences: impl IntoIterator<Item = Sequence>,
    ) -> EntityCommands<'_>;
}

impl BuildPlayerAppExt for Commands<'_, '_> {
    fn create_sequence_player(
        &mut self,
        sequences: impl IntoIterator<Item = Sequence>,
    ) -> EntityCommands<'_> {
        let player_id = self.spawn_empty().id();

        // let player = SequencePlayer {
        //     sequence_ids: sequences
        //         .into_iter()
        //         .map(|s| {
        //             self.spawn((s, TargetPlayer(player_id))).id()
        //         })
        //         .collect(),
        //     ..default()
        // };

        for sequence in sequences {
            self.spawn((sequence, TargetPlayer(player_id)));
        }

        // self.entity(player_id).insert(player);
        self.entity(player_id)
    }
}

/// Update [`SequenceController::target_time`] based on [`SequencePlayer`].
fn update_target_time(
    q_players: Query<&SequencePlayer>,
    mut q_sequences: Query<(&Sequence, &mut SequenceController)>,
    time: Res<Time>,
) -> Result {
    for player in q_players.iter() {
        // No movement is needed...
        if player.time_scale == 0.0 || player.is_playing == false {
            continue;
        }

        let (sequence, mut controller) =
            q_sequences.get_mut(player.curr_sequence_id())?;

        // When time scale indicates moving forward and we've reached the end.
        let reached_end = player.time_scale > 0.0
            && controller.curr_time() >= sequence.duration();
        // When time scale indicates moving backward and we've reached the start.
        let reached_start =
            player.time_scale < 0.0 && controller.curr_time() <= 0.0;

        if reached_end || reached_start {
            continue;
        }

        controller.target_time +=
            player.time_scale * time.delta_secs();
    }

    Ok(())
}

fn jump_sequence(
    trigger: Trigger<JumpSequence>,
    mut q_players: Query<&mut SequencePlayer>,
    mut q_sequences: Query<(&Sequence, &mut SequenceController)>,
) -> Result {
    let player_id = trigger.target();
    let jump = trigger.event();

    let mut player = q_players.get_mut(player_id)?;
    let target_index = jump.index.min(player.sequence_ids.len() - 1);

    if target_index != player.sequence_index {
        // Fast-forward or rewind sequences that have been
        // skipped over to set their final state.
        let is_forward = target_index > player.sequence_index;
        let (mut min, mut max) = (
            target_index.min(player.sequence_index),
            target_index.max(player.sequence_index),
        );

        if is_forward == false {
            // Shift indices forward to prevent altering
            // the target sequence.
            max += 1;
            min += 1;
        }

        let set_target_time = if is_forward {
            // Set to the end if the index is moving forward.
            |sequence: &Sequence,
             controller: &mut SequenceController| {
                controller.target_time = sequence.duration();
            }
        } else {
            // Set to the start if the index is moving backward.
            |_: &Sequence, controller: &mut SequenceController| {
                controller.target_time = 0.0;
            }
        };

        for i in min..max {
            let sequence_id = player.sequence_ids[i];
            let (sequence, mut controller) =
                q_sequences.get_mut(sequence_id)?;

            // Set the target time based on the conditioned closure.
            set_target_time(sequence, &mut controller);
        }
    }

    // Apply jump configuration to the player.
    player.sequence_index = target_index;
    player.time_scale = jump.time_scale;

    // Apply the waypoint to the target sequence.
    let (sequence, mut controller) =
        q_sequences.get_mut(player.curr_sequence_id())?;

    match jump.waypoint {
        Waypoint::Start => controller.target_time = 0.0,
        Waypoint::End => controller.target_time = sequence.duration(),
    }

    Ok(())
}

#[derive(Event)]
pub struct JumpSequence {
    pub index: usize,
    /// The `time_scale` that the player should play after jumping
    /// to the target [`Sequence`].
    pub time_scale: f32,
    /// Deteremines the starting point of the [`Sequence`].
    pub waypoint: Waypoint,
}

/// Manipulates [`SequenceController::target_time`].
#[derive(Component, Default)]
#[relationship_target(relationship = TargetPlayer, linked_spawn)]
pub struct SequencePlayer {
    /// The [`Sequence`]s that is related to this player.
    #[relationship]
    sequence_ids: SmallVec<[Entity; 1]>,
    /// Whether to play or pause the sequence.
    is_playing: bool,
    /// The speed of the playback time.
    time_scale: f32,
    /// The index in `sequence_ids`.
    sequence_index: usize,
}

impl SequencePlayer {
    /// Get the current sequence id based on `sequence_index`.
    fn curr_sequence_id(&self) -> Entity {
        self.sequence_ids[self.sequence_index]
    }

    #[inline]
    pub fn with_playing(mut self, is_playing: bool) -> Self {
        self.is_playing = is_playing;
        self
    }

    #[inline]
    pub fn with_time_scale(mut self, time_scale: f32) -> Self {
        self.time_scale = time_scale;
        self
    }

    #[inline]
    pub fn play(&mut self) -> &mut Self {
        self.is_playing = true;
        self
    }

    #[inline]
    pub fn pause(&mut self) -> &mut Self {
        self.is_playing = false;
        self
    }

    #[inline]
    pub fn set_time_scale(&mut self, time_scale: f32) -> &mut Self {
        self.time_scale = time_scale;
        self
    }

    // #[inline]
    // pub fn set_index(&mut self, target_index: usize) -> &mut Self {
    //     self.target_index =
    //         target_index.min(self.sequence_ids.len() - 1);
    //     self
    // }
}

/// The target [`SequencePlayer`] that this [`Sequence`] belongs to.
#[derive(
    Component, Reflect, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
#[reflect(Component)]
#[relationship(relationship_target = SequencePlayer)]
pub struct TargetPlayer(Entity);

/// Deteremines where the starting point should be when jumping
/// to another [`Sequence`].
#[derive(Clone, Copy)]
pub enum Waypoint {
    Start,
    End,
}

// #[derive(Bundle, Default)]
// pub struct SlideBundle {
//     pub sequence: Sequence,
//     pub sequence_controller: SequenceController,
//     pub slide_controller: SlideController,
// }

// #[derive(Component, Clone)]
// pub struct SlideController {
//     /// Start time of all slides including 1 extra at the end
//     /// that represents the duration of the entire sequence.
//     start_times: Vec<f32>,
//     target_slide_index: usize,
//     curr_state: SlideCurrState,
//     target_state: SlideTargetState,
//     time_scale: f32,
// }

// impl SlideController {
//     pub fn next(&mut self) {
//         match self.curr_state {
//             SlideCurrState::End => {
//                 self.target_slide_index = usize::min(
//                     self.target_slide_index + 1,
//                     self.slide_count() - 1,
//                 );
//             }
//             _ => {
//                 self.target_state = SlideTargetState::End;
//             }
//         }
//     }

//     pub fn prev(&mut self) {
//         match self.curr_state {
//             SlideCurrState::Start => {
//                 self.target_slide_index =
//                     self.target_slide_index.saturating_sub(1);
//             }
//             _ => {
//                 self.target_state = SlideTargetState::Start;
//             }
//         }
//     }

//     pub fn seek(
//         &mut self,
//         slide_index: usize,
//         slide_state: SlideTargetState,
//     ) {
//         self.target_slide_index =
//             usize::min(slide_index, self.slide_count() - 1);
//         self.target_state = slide_state;
//     }

//     #[inline]
//     pub fn set_time_scale(&mut self, time_scale: f32) {
//         self.time_scale = f32::abs(time_scale);
//     }

//     #[inline]
//     pub fn slide_count(&self) -> usize {
//         self.start_times.len().saturating_sub(1)
//     }
// }

// impl Default for SlideController {
//     fn default() -> Self {
//         Self {
//             start_times: Vec::default(),
//             target_slide_index: 0,
//             curr_state: SlideCurrState::default(),
//             target_state: SlideTargetState::default(),
//             time_scale: 1.0,
//         }
//     }
// }

// #[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
// pub enum SlideCurrState {
//     #[default]
//     Start,
//     Mid,
//     End,
// }

// #[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
// pub enum SlideTargetState {
//     #[default]
//     Start,
//     End,
// }

// pub fn create_slide(mut sequences: Vec<Sequence>) -> SlideBundle {
//     let mut start_times = Vec::with_capacity(sequences.len());

//     let mut start_time = 0.0;
//     for (s, sequence) in sequences.iter_mut().enumerate() {
//         sequence.set_slide_index(s as u32);
//         start_times.push(start_time);

//         start_time += sequence.duration();
//     }
//     start_times.push(start_time);

//     SlideBundle {
//         sequence: sequences.chain(),
//         slide_controller: SlideController {
//             start_times,
//             ..default()
//         },
//         ..default()
//     }
// }

// pub(crate) fn slide_controller(
//     mut q_slides: Query<(
//         &mut SlideController,
//         &mut SequenceController,
//     )>,
//     time: Res<Time>,
// ) {
//     for (mut slide_controller, mut sequence_controller) in
//         q_slides.iter_mut()
//     {
//         if slide_controller.time_scale <= f32::EPSILON {
//             continue;
//         }

//         // Determine direction based on target slide state. (it can only be start or end)
//         let direction = {
//             match slide_controller.target_state {
//                 SlideTargetState::Start => -1,
//                 SlideTargetState::End => 1,
//             }
//         };

//         // Update sequence target time and target slide index
//         sequence_controller.target_time += time.delta_secs()
//             * slide_controller.time_scale
//             * direction as f32;
//         sequence_controller.target_slide =
//             slide_controller.target_slide_index;

//         // Initialize as mid
//         slide_controller.curr_state = SlideCurrState::Mid;

//         // Clamp target time based on direction
//         if direction < 0 {
//             let start_time = slide_controller.start_times
//                 [sequence_controller.target_slide];

//             // Start time reached
//             if sequence_controller.target_time <= start_time {
//                 slide_controller.curr_state = SlideCurrState::Start;
//                 sequence_controller.target_time = start_time;
//             }
//         } else {
//             let end_time = slide_controller.start_times
//                 [sequence_controller.target_slide + 1];

//             // End time reached
//             if sequence_controller.target_time >= end_time {
//                 slide_controller.curr_state = SlideCurrState::End;
//                 sequence_controller.target_time = end_time;
//             }
//         }
//     }
// }
