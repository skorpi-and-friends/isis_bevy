use deps::*;

use crate::math::*;
use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

pub mod attire;
pub mod engine;

pub struct CraftsPlugin;
impl Plugin for CraftsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(engine::sync_craft_state_velocities.system())
            .add_system(engine::linear_pid_driver.system())
            .add_system(engine::angular_pid_driver.system())
            .add_system(engine::apply_flames_simple_accel.system())
            .add_system(
                attire::generate_better_contact_events
                    .system()
                    .label(attire::AttireSystems::GenerateBetterContactEvents),
            )
            .add_system(
                attire::handle_collision_damage_events
                    .system()
                    .after(attire::AttireSystems::GenerateBetterContactEvents),
            )
            .add_system(attire::log_damage_events.system())
            .add_event::<attire::BetterContactEvent>()
            .add_event::<attire::CollisionDamageEvent>();
    }
}

pub struct CraftCamera;

pub struct CurrentCraft(pub Entity);

#[derive(Bundle)]
pub struct CraftBundle {
    pub xfrom: Transform,
    pub global_xform: GlobalTransform,

    #[bundle]
    pub rigid_body: RigidBodyBundle,

    pub rigid_body_sync: RigidBodyPositionSync,
    pub collision_damage_tag: attire::CollisionDamageEnabledRb,

    pub config: engine::EngineConfig,
    pub linear_state: engine::LinearEngineState,
    pub angular_state: engine::AngularEngineState,
    pub linear_pid: engine::LinearDriverPid,
    pub angular_pid: engine::AngularDriverPid,
}

impl Default for CraftBundle {
    fn default() -> Self {
        Self {
            xfrom: Transform::default(),
            global_xform: GlobalTransform::default(),
            config: Default::default(),
            linear_state: Default::default(),
            angular_state: Default::default(),
            linear_pid: engine::LinearDriverPid(crate::utils::PIDControllerVec3::new(
                Vector3::ONE * 1000.,
                Vector3::ZERO,
                Vector3::ZERO,
                Vector3::ZERO,
                Vector3::ZERO,
            )),
            angular_pid: engine::AngularDriverPid(crate::utils::PIDControllerVec3::new(
                Vector3::ONE * 1000.0,
                Vector3::ZERO,
                Vector3::ZERO,
                Vector3::ZERO,
                Vector3::ZERO,
            )),
            rigid_body: RigidBodyBundle {
                ccd: RigidBodyCcd {
                    ccd_active: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            rigid_body_sync: RigidBodyPositionSync::Discrete,
            collision_damage_tag: attire::CollisionDamageEnabledRb,
        }
    }
}
