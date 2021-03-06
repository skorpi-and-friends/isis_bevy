use deps::*;

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::craft::CraftDimensions;
use crate::math::*;

#[derive(Debug, Default, Clone, Component, Reflect, Inspectable)]
pub struct LinearEngineState {
    /// Linear velocity in local-space
    /// In m/s.
    pub velocity: TVec3,

    /// Input vector for driver. Meaning depends on driver implementation.
    /// e.g. target velocity to attain
    pub input: TVec3,

    /// Vector output of driver and input vector of a motor. Meaning depends on implementation.
    /// e.g. forve to apply
    pub flame: TVec3,
}

#[derive(Debug, Default, Clone, Component, Reflect, Inspectable)]
pub struct AngularEngineState {
    /// Angular velocity in local-space
    /// In rad/s.
    pub velocity: TVec3,
    /// Input vector for driver. Meaning depends on driver implementation.
    /// e.g. target velocity to attain
    pub input: TVec3,
    /// Vector output of driver and input vector of a motor. Meaning depends on implementation.
    pub flame: TVec3,
}

// TODO: break this up to multiple components. Maybe along the line of what's likely to mutate?
#[derive(Debug, Clone, Component, Reflect, Inspectable)]
pub struct EngineConfig {
    ///  Speed to travel at when there is no input i.e. how fast to travel when idle.
    pub set_speed: TVec3,

    /// Total mass of the craft.
    /// In KG.
    pub mass: TReal,

    /// Maximum acceleration allowed to the craft.
    /// In m/s.
    pub acceleration_limit: TVec3,

    pub acceleration_limit_multiplier: TReal,

    /// Linear velocity cap no matter the input.
    /// In m/s.
    pub linvel_limit: TVec3,

    /// Angular velocity cap no matter the input.
    /// In rad/s.
    pub angvel_limit: TVec3,

    /// Max force the linear thrusters are capable of exerting.
    /// In Newtons.
    pub linear_thruster_force: TVec3,

    /// Whether or not to respect linvel_limit in the z axis.
    pub limit_forward_v: bool,

    /// Whether or not to respect linvel_limit in in the X or Y axis.
    pub limit_strafe_v: bool,

    /// Whether or not to respect angvel_limit.
    pub limit_angular_v: bool,

    ///  Whether or not to respect acceleration_limit.
    pub limit_acceleration: bool,

    /// Max force the angular thrusters are capable of exerting.
    /// In Newtons.
    pub angular_thruster_force: TVec3,

    pub thruster_force_multiplier: TReal,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mass: 15_000.,
            set_speed: TVec3::ZERO,
            acceleration_limit: [6., 6., 6.].into(),
            acceleration_limit_multiplier: 9.81,
            // matters not if v_limit.z is negative since this's a limit
            linvel_limit: [100., 100., 200.].into(),
            angvel_limit: [3., 3., 3.].into(),
            limit_forward_v: true,
            limit_strafe_v: true,
            limit_angular_v: true,
            limit_acceleration: true,
            linear_thruster_force: [1., 1., 1.5].into(),
            angular_thruster_force: [1., 1., 1.].into(),
            thruster_force_multiplier: 1_000_000.0,
        }
    }
}

impl EngineConfig {
    /// Use this everytime the [`EngineConfig`] or [`CraftDimensions`] changes to calculate transiet items.
    pub fn derive_items(&self, dimensions: CraftDimensions) -> DerivedEngineConfig {
        use bevy::math::vec2;
        let axes_diameter: TVec3 = [
            vec2(dimensions.y, dimensions.z).length(),
            vec2(dimensions.x, dimensions.z).length(),
            vec2(dimensions.x, dimensions.y).length(),
        ]
        .into();

        DerivedEngineConfig {
            angular_acceleration_limit: [TReal::INFINITY; 3].into(),
            thruster_torque: axes_diameter * self.angular_thruster_force,
        }
    }

    #[inline]
    pub fn actual_acceleration_limit(&self) -> TVec3 {
        self.acceleration_limit * self.acceleration_limit_multiplier
    }

    /// This doesn't take into account the [`acceleration_limit`]. Clapmp it yourself.
    #[inline]
    pub fn avail_lin_accel(&self) -> TVec3 {
        let max_force = self.linear_thruster_force * self.thruster_force_multiplier;
        max_force / self.mass
    }
}

#[derive(Debug, Clone, Component, Reflect, Inspectable)]
pub struct DerivedEngineConfig {
    /// Angular thruster toruqe, transient auto cacluated value from the
    /// angular_thrustuer_force according to the craft's shape and mass.
    /// In  Newton meters.
    pub thruster_torque: TVec3,

    /// Angular acceleration limit, another transient auto cacluated value. It's cacluated from
    /// the normal acceleration limit (which is in m/ss) and adjusted to the size/shape of the craft.
    /// In rad/s/s.
    ///
    /// Curretly unused. Defaults to INFINITY meaning there's no artifical acceleration_limit on
    /// the crafts. They use all of what's availaible from the thrusters.
    pub angular_acceleration_limit: TVec3,
    ///// Moment of inertia, transient auto cacluated value used to convert the required angular
    ///// acceleration into the appropriate torque. Aquried directly from Godot's physics engine.
    ///// In  kg*m*m.
    ///// Defaults to one to avoid hard to track division by zero errors. The moi is asychronously
    ///// retrieved from the engine and some frames pass before it happens. Time enough for the NANs
    ///// to propagate EVERYWHERE!
    //pub moment_of_inertia: Vector3,
}

#[derive(Debug, Component)]
pub struct LinearDriverPid(pub crate::utils::PIDControllerVec3);
#[derive(Debug, Component)]
pub struct AngularDriverPid(pub crate::utils::PIDControllerVec3);

pub fn sync_craft_state_velocities(
    mut crafts: Query<(
        &mut AngularEngineState,
        &mut LinearEngineState,
        &GlobalTransform,
        &RigidBodyVelocityComponent,
    )>,
) {
    for (mut angular_state, mut linear_state, g_xform, rb_velocity) in crafts.iter_mut() {
        // convert it to local space first
        let rotator = g_xform.rotation.inverse();
        angular_state.velocity = rotator * TVec3::from(rb_velocity.angvel);
        linear_state.velocity = rotator * TVec3::from(rb_velocity.linvel);
    }
}

pub fn linear_pid_driver(
    mut crafts: Query<(&mut LinearEngineState, &EngineConfig, &mut LinearDriverPid)>,
    //time: Time,
) {
    for (mut state, config, mut pid) in crafts.iter_mut() {
        let mut linear_input = state.input;

        // if dampeners are on
        if config.limit_strafe_v {
            let v_limit = config.linvel_limit;

            // clamp the input to the limit
            linear_input = linear_input.clamp(-v_limit, v_limit);

            if !config.limit_forward_v {
                linear_input.z = state.input.z;
            }
        }

        // calculate max acceleration possible using availaible force
        let acceleration_limit = {
            let mut acceleration_limit = config.avail_lin_accel();

            // NOTE: fwd is negative bc rh coord sys
            let move_fwd = linear_input.z < 0.0;

            // if input wants to go bacwards
            if !move_fwd {
                // only use starfe thrusters force on the z
                acceleration_limit.z = acceleration_limit.x.max(acceleration_limit.y);
            }

            if config.limit_acceleration {
                let artificial_accel_limit = config.actual_acceleration_limit();

                // clamp the actual limit to the artifical limit
                acceleration_limit.clamp(-artificial_accel_limit, artificial_accel_limit)
            } else {
                acceleration_limit
            }
        };

        let linear_flame = pid
            .0
            .update(state.velocity, linear_input - state.velocity, 1.);

        state.flame = linear_flame.clamp(-acceleration_limit, acceleration_limit);
    }
}

pub fn angular_pid_driver(
    mut crafts: Query<(
        &mut AngularEngineState,
        &EngineConfig,
        &DerivedEngineConfig,
        &mut AngularDriverPid,
        &RigidBodyMassPropsComponent,
    )>,
    time: Res<Time>,
) {
    for (mut state, config, derived_config, mut pid, mass_props) in crafts.iter_mut() {
        {
            let angular_input = if config.limit_angular_v {
                state.input.clamp(-config.angvel_limit, config.angvel_limit)
            } else {
                state.input
            };

            let acceleration_limit = {
                let max_torque = derived_config.thruster_torque * config.thruster_force_multiplier;

                // TODO: work out if this is actually the inertia tensor
                let local_moi_inv_sqrt = mass_props.local_mprops.inv_principal_inertia_sqrt;

                // NOTICE: difference here
                let acceleration_limit: TVec3 = [
                    max_torque.x * local_moi_inv_sqrt.x,
                    max_torque.y * local_moi_inv_sqrt.y,
                    max_torque.z * local_moi_inv_sqrt.z,
                ]
                .into();
                if config.limit_acceleration {
                    let artificial_accel_limit = derived_config.angular_acceleration_limit;
                    pid.0.integrat_max = acceleration_limit.min(artificial_accel_limit);
                    pid.0.integrat_min = -pid.0.integrat_max;

                    // clamp the actual limit to the artifical limit

                    acceleration_limit.clamp(-artificial_accel_limit, artificial_accel_limit)
                } else {
                    acceleration_limit
                }
            };

            let angular_flame = pid.0.update(
                state.velocity,
                angular_input - state.velocity,
                time.delta_seconds(),
            );
            // let angular_flame = angular_input * ;
            state.flame = angular_flame.clamp(-acceleration_limit, acceleration_limit);
        }
    }
}

pub fn apply_flames_simple_accel(
    mut crafts: Query<(
        &GlobalTransform,
        &LinearEngineState,
        &AngularEngineState,
        &EngineConfig,
        &RigidBodyMassPropsComponent,
        &mut RigidBodyForcesComponent,
    )>,
    //time: Time,
) {
    for (g_xform, lin_state, ang_state, config, mass_props, mut forces) in crafts.iter_mut() {
        let force = lin_state.flame * config.mass;
        let force = g_xform.rotation * force;
        forces.force += Vector::from(force);

        let local_moi_inv_sqrt = mass_props.local_mprops.inv_principal_inertia_sqrt;
        let torque: TVec3 = [
            ang_state.flame.x / local_moi_inv_sqrt.x,
            ang_state.flame.y / local_moi_inv_sqrt.y,
            ang_state.flame.z / local_moi_inv_sqrt.z,
        ]
        .into();
        let torque = g_xform.rotation * torque;

        forces.torque += AngVector::from(torque);
    }
}
