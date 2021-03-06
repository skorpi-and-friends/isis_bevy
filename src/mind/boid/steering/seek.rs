use deps::*;

use super::{
    steering_behaviours, ActiveSteeringRoutine, LinOnlyRoutineBundle, LinearRoutineOutput,
    SteeringRoutine,
};
use crate::math::*;
use bevy::prelude::*;

#[derive(Debug, Clone, Component)]
pub enum Target {
    /// must have a global xform
    Object { entt: Entity },
    /// assumed to be in world basis
    Position { pos: TVec3 },
}

#[derive(Debug, Clone, Component)]
pub struct Seek {
    pub target: Target,
}

pub type Bundle = LinOnlyRoutineBundle<Seek>;

pub fn update(
    mut routines: Query<
        (&Seek, &SteeringRoutine, &mut LinearRoutineOutput),
        With<ActiveSteeringRoutine>,
    >,
    objects: Query<&GlobalTransform>,
) {
    for (param, routine, mut output) in routines.iter_mut() {
        let xform = objects
            .get(routine.boid_entt)
            .expect_or_log("craft entt not found for routine");
        let pos = match param.target {
            Target::Object { entt } => objects.get(entt).unwrap_or_log().translation,
            Target::Position { pos } => pos,
        };
        *output = steering_behaviours::seek_position(xform.translation, pos).into();
    }
}
