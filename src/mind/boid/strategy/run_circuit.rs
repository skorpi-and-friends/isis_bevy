use deps::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{
    craft::*,
    mind::{boid::*, *},
};

use super::{ActiveBoidStrategy, BoidStrategy, BoidStrategyBundleExtra, BoidStrategyOutput};

#[derive(Debug, Clone, Component)]
pub struct RunCircuit {
    pub initial_point: Entity,
}

#[derive(Debug, Clone, Component)]
pub struct CircuitWaypoint {
    pub next_point: Entity,
}

#[derive(Debug, Clone, Component, Default)]
pub struct RunCircuitState {
    pub composer_routine: Option<Entity>,
    pub arrive_routine: Option<Entity>,
    pub avoid_collision_routine: Option<Entity>,
}

pub type Bundle = BoidStrategyBundleExtra<RunCircuit, RunCircuitState>;

pub fn butler(
    mut commands: Commands,
    mut added_strategies: Query<
        (
            Entity,
            &RunCircuit,
            &BoidStrategy,
            &mut RunCircuitState,
            &mut BoidStrategyOutput,
        ),
        Added<RunCircuit>,
    >,
    waypoints: Query<(&CircuitWaypoint, &GlobalTransform)>,
    crafts: Query<(
        &sensors::SteeringRoutinesIndex,
        &engine::EngineConfig,
        &CraftDimensions,
    )>,
) {
    for (entt, param, strategy, mut state, mut out) in added_strategies.iter_mut() {
        let (routines, engine_config, dim) = crafts
            .get(strategy.boid_entt())
            .expect_or_log("craft not found for BoidStrategy");
        let (_, waypoint1_xform) = waypoints.get(param.initial_point).unwrap_or_log();
        // let (_, waypoint2_xform) = waypoints.get(waypoint1.next_point).unwrap_or_log();

        let raycast_toi_modifier = dim.max_element();
        let cast_shape_radius = raycast_toi_modifier * 0.5;
        let avoid_collision = routines
            .kind::<avoid_collision::AvoidCollision>()
            .map(|v| v[0])
            .unwrap_or_else(|| {
                commands
                    .spawn()
                    .insert_bundle(avoid_collision::Bundle::new(
                        avoid_collision::AvoidCollision::new(
                            cast_shape_radius,
                            raycast_toi_modifier,
                        ),
                        strategy.boid_entt(),
                        Default::default(),
                    ))
                    .id()
            });
        let arrive = commands
            .spawn()
            .insert_bundle(arrive::Bundle::new(
                arrive::Arrive {
                    target: arrive::Target::Vector {
                        at_pos: waypoint1_xform.translation,
                        pos_linvel: Default::default(),
                        // with_linvel: Default::default(),
                        /* with_linvel: (waypoint2_xform.translation - waypoint1_xform.translation)
                        .normalize()
                        * engine_config.linvel_limit, */
                        with_speed: 80.,
                    },
                    arrival_tolerance: 5.,
                    deceleration_radius: None,
                    linvel_limit: engine_config.linvel_limit,
                    avail_accel: engine_config.avail_lin_accel().clamp(
                        -engine_config.actual_acceleration_limit(),
                        engine_config.actual_acceleration_limit(),
                    ),
                },
                strategy.boid_entt(),
            ))
            .id();
        let compose = commands
            .spawn()
            .insert_bundle(compose::Bundle::new(
                compose::Compose {
                    composer: compose::SteeringRoutineComposer::PriorityOverride {
                        routines: smallvec::smallvec![avoid_collision, arrive],
                    },
                },
                strategy.boid_entt(),
            ))
            .id();

        state.arrive_routine = Some(arrive);
        state.avoid_collision_routine = Some(avoid_collision);
        state.composer_routine = Some(compose);
        *out = BoidStrategyOutput {
            steering_routine: Some(compose),
            fire_weapons: false,
        };

        commands.entity(entt).insert(ActiveBoidStrategy);
    }
}

pub fn update(
    // mut commands: Commands,
    strategies: Query<&RunCircuitState, With<ActiveBoidStrategy>>,
    waypoints: Query<(Entity, &CircuitWaypoint, &GlobalTransform)>,
    narrow_phase: Res<NarrowPhase>,
    parents: Query<&ColliderParentComponent>,
    mut arrive_routines: Query<&mut arrive::Arrive>,
    crafts: Query<(
        // &engine::EngineConfig,
        &sensors::BoidStrategyIndex,
        &CraftDimensions,
        &RigidBodyVelocityComponent,
    )>,
) {
    for (checkpt_entt, waypoint, checkopoint_xform) in waypoints.iter() {
        // if something triggered the waypoint
        for (collider1, collider2) in narrow_phase
            .intersections_with(checkpt_entt.handle())
            .filter(|(_, _, ixing)| *ixing)
            .map(|(c1, c2, _)| (c1, c2))
        {
            let other_collider = if collider1.entity() == checkpt_entt {
                collider2.entity()
            } else {
                collider1.entity()
            };
            if let Ok(Ok((index, dim, vel))) = parents
                // if other_collider has a rigd body
                .get(other_collider)
                // and that rigd body belongs to a craft
                .map(|parent| crafts.get(parent.handle.entity()))
            {
                // for any acttive RunCircuit strategies on the craft
                if let Some(entts) = index.kind::<RunCircuit>() {
                    for entt in entts {
                        // BUG: implement a measure for active strategy check
                        let state = strategies.get(*entt).unwrap_or_log();
                        let mut arrive_param = arrive_routines
                            .get_mut(state.arrive_routine.unwrap_or_log())
                            .unwrap_or_log();
                        match arrive_param.target {
                            arrive::Target::Vector {
                                at_pos: prev_pos,
                                with_speed,
                                ..
                            } => {
                                if prev_pos.distance_squared(checkopoint_xform.translation)
                                    - (dim.max_element().powi(2))
                                    < 1.
                                {
                                    let cur_vel = vel.linvel;
                                    let cur_spd = cur_vel.magnitude();
                                    // commands.entity(other_collider).despawn_recursive();
                                    tracing::info!(
                                        ?cur_vel,
                                        ?cur_spd,
                                        "craft arrived at waypoint {prev_pos:?}",
                                    );
                                    let (_, _, next_waypoint_xform) =
                                        waypoints.get(waypoint.next_point).unwrap_or_log();

                                    /* let (_, _, next_next_waypoint_xform) = waypoints
                                    .get(next_waypoint.next_point)
                                    unwrap_or_log(); */
                                    arrive_param.target = arrive::Target::Vector {
                                        at_pos: next_waypoint_xform.translation,
                                        pos_linvel: TVec3::ZERO,
                                        // with_linvel: TVec3::ZERO,
                                        /* with_linvel: (next_next_waypoint_xform.translation
                                        - next_waypoint_xform.translation)
                                        .normalize()
                                        * engine_config.linvel_limit, */
                                        with_speed,
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
