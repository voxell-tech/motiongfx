use bevy::prelude::*;

use crate::prelude::{MultiSeqOrd, Sequence, SequenceController};

#[derive(Bundle, Default)]
pub struct SlideBundle {
    pub sequence: Sequence,
    pub sequence_controller: SequenceController,
    pub slide_controller: SlideController,
}

#[derive(Component, Clone)]
pub struct SlideController {
    /// Start time of all slides including 1 extra at the end that represents the duration of the entire sequence.
    start_times: Vec<f32>,
    target_slide_index: usize,
    curr_state: SlideCurrState,
    target_state: SlideTargetState,
    utime_scale: f32,
}

impl SlideController {
    pub fn next(&mut self) {
        match self.curr_state {
            SlideCurrState::End => {
                self.target_slide_index =
                    usize::min(self.target_slide_index + 1, self.slide_count() - 1);
            }
            _ => {
                self.target_state = SlideTargetState::End;
            }
        }
    }

    pub fn prev(&mut self) {
        match self.curr_state {
            SlideCurrState::Start => {
                self.target_slide_index = self.target_slide_index.saturating_sub(1);
            }
            _ => {
                self.target_state = SlideTargetState::Start;
            }
        }
    }

    pub fn seek(&mut self, slide_index: usize, slide_state: SlideTargetState) {
        self.target_slide_index = usize::min(slide_index, self.slide_count() - 1);
        self.target_state = slide_state;
    }

    #[inline]
    pub fn set_time_scale(&mut self, time_scale: f32) {
        self.utime_scale = f32::abs(time_scale);
    }

    #[inline]
    pub fn slide_count(&self) -> usize {
        self.start_times.len().saturating_sub(1)
    }
}

impl Default for SlideController {
    fn default() -> Self {
        Self {
            start_times: Vec::default(),
            target_slide_index: 0,
            curr_state: SlideCurrState::default(),
            target_state: SlideTargetState::default(),
            utime_scale: 1.0,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlideCurrState {
    #[default]
    Start,
    Mid,
    End,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlideTargetState {
    #[default]
    Start,
    End,
}

pub fn create_slide(mut sequences: Vec<Sequence>) -> SlideBundle {
    let mut start_times = Vec::with_capacity(sequences.len());

    let mut start_time = 0.0;
    for (s, sequence) in sequences.iter_mut().enumerate() {
        sequence.set_slide_index(s);
        start_times.push(start_time);

        start_time += sequence.duration();
    }
    start_times.push(start_time);

    SlideBundle {
        sequence: sequences.chain(),
        slide_controller: SlideController {
            start_times,
            ..default()
        },
        ..default()
    }
}

pub(crate) fn slide_controller(
    mut q_slides: Query<(&mut SlideController, &mut SequenceController)>,
    time: Res<Time>,
) {
    for (mut slide_controller, mut sequence_controller) in q_slides.iter_mut() {
        if slide_controller.utime_scale <= f32::EPSILON {
            continue;
        }

        // Determine direction based on target slide state. (it can only be start or end)
        let direction = {
            match slide_controller.target_state {
                SlideTargetState::Start => -1,
                SlideTargetState::End => 1,
            }
        };

        // Update sequence target time and target slide index
        sequence_controller.target_time +=
            time.delta_secs() * slide_controller.utime_scale * direction as f32;
        sequence_controller.target_slide_index = slide_controller.target_slide_index;

        // Initialize as mid
        slide_controller.curr_state = SlideCurrState::Mid;

        // Clamp target time based on direction
        if direction < 0 {
            let start_time = slide_controller.start_times[sequence_controller.target_slide_index];

            // Start time reached
            if sequence_controller.target_time <= start_time {
                slide_controller.curr_state = SlideCurrState::Start;
                sequence_controller.target_time = start_time;
            }
        } else {
            let end_time = slide_controller.start_times[sequence_controller.target_slide_index + 1];

            // End time reached
            if sequence_controller.target_time >= end_time {
                slide_controller.curr_state = SlideCurrState::End;
                sequence_controller.target_time = end_time;
            }
        }
    }
}
