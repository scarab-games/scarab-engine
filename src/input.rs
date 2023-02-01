use piston::{Button, ButtonArgs, ButtonState, Input};
use serde::{Deserialize, Serialize};

use crate::ScarabResult;

/// A trait for types that handle user inputs.
/// User input handling is split into two stages: mapping input to action and performing the action
/// This division is intended to allow for a more intuitive divide between parsing the inputs and
/// actually doing the things they're intended to do.
pub trait InputRegistry {
    /// The different actions that the registry can handle
    type InputActions;
    /// What the input action should act upon
    type InputTarget;

    fn do_input_action(
        &self,
        action: Self::InputActions,
        target: &mut Self::InputTarget,
    ) -> ScarabResult<()>;

    fn map_input_to_action(&mut self, input: Input) -> Option<Self::InputActions>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Axis2dBinding {
    pos_x: (Button, f64),
    neg_x: (Button, f64),
    pos_y: (Button, f64),
    neg_y: (Button, f64),
}

impl Axis2dBinding {
    pub fn new(pos_x: Button, pos_y: Button, neg_x: Button, neg_y: Button) -> Self {
        Self {
            pos_x: (pos_x, 0.0),
            pos_y: (pos_y, 0.0),
            neg_x: (neg_x, 0.0),
            neg_y: (neg_y, 0.0),
        }
    }

    fn set_axis_button(&mut self, button: ButtonState, dir: Axis2dDirection) {
        let val = match button {
            ButtonState::Press => 1.0,
            ButtonState::Release => 0.0,
        };
        self.set_axis(val, dir)
    }

    fn set_axis(&mut self, val: f64, dir: Axis2dDirection) {
        match dir {
            Axis2dDirection::PosX => self.pos_x.1 = val,
            Axis2dDirection::NegX => self.neg_x.1 = val,
            Axis2dDirection::PosY => self.pos_y.1 = val,
            Axis2dDirection::NegY => self.neg_y.1 = val,
        }
    }

    fn maybe_direction_from_button(&self, args: &ButtonArgs) -> Option<Axis2dDirection> {
        if args.button == self.pos_x.0 {
            Some(Axis2dDirection::PosX)
        } else if args.button == self.pos_y.0 {
            Some(Axis2dDirection::PosY)
        } else if args.button == self.neg_x.0 {
            Some(Axis2dDirection::NegX)
        } else if args.button == self.neg_y.0 {
            Some(Axis2dDirection::NegY)
        } else {
            None
        }
    }

    pub fn maybe_to_action(&mut self, args: ButtonArgs) -> Option<[f64; 2]> {
        if let Some(dir) = self.maybe_direction_from_button(&args) {
            self.set_axis_button(args.state, dir);
            Some(self.into())
        } else {
            None
        }
    }
}

impl From<Axis2dBinding> for [f64; 2] {
    fn from(val: Axis2dBinding) -> Self {
        [val.pos_x.1 - val.neg_x.1, val.pos_y.1 - val.neg_y.1]
    }
}

impl From<&Axis2dBinding> for [f64; 2] {
    fn from(val: &Axis2dBinding) -> Self {
        [val.pos_x.1 - val.neg_x.1, val.pos_y.1 - val.neg_y.1]
    }
}

impl From<&mut Axis2dBinding> for [f64; 2] {
    fn from(val: &mut Axis2dBinding) -> Self {
        [val.pos_x.1 - val.neg_x.1, val.pos_y.1 - val.neg_y.1]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Axis2dDirection {
    PosX,
    NegX,
    PosY,
    NegY,
}
