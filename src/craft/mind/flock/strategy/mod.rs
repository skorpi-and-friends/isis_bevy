use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

pub use cas::*;
mod cas;

#[derive(Debug, Clone, Component)]
pub struct ActiveFlockStrategy {
    pub strategy: Entity,
}

/// A generic bundle for flock strategies.
#[derive(Bundle)]
pub struct FlockStrategyBundle<P>
where
    P: Component,
{
    pub param: P,
    pub tag: FlockStrategy,
}

impl<P> FlockStrategyBundle<P>
where
    P: Component,
{
    pub fn new(param: P, flock_entt: Entity) -> Self {
        Self {
            param,
            tag: FlockStrategy::new(flock_entt, FlockStrategyKind::of::<P>()),
        }
    }
}

/// A variant of [`FlockStrategyBundle`] with two parameter components.
#[derive(Bundle)]
pub struct FlockStrategyBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub param: P,
    pub extra: P2,
    pub tag: FlockStrategy,
}

impl<P, P2> FlockStrategyBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub fn new(param: P, flock_entt: Entity, extra: P2) -> Self {
        Self {
            param,
            extra,
            tag: FlockStrategy::new(flock_entt, FlockStrategyKind::of::<P>()),
        }
    }
}

/// A variant of [`FlockStrategyBundleExtra`] where the second component is also a bundle.
#[derive(Bundle)]
pub struct FlockStrategyBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub param: P,
    #[bundle]
    pub extra: B,
    pub tag: FlockStrategy,
}

impl<P, B> FlockStrategyBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub fn new(param: P, flock_entt: Entity, extra: B) -> Self {
        Self {
            param,
            extra,
            tag: FlockStrategy::new(flock_entt, FlockStrategyKind::of::<P>()),
        }
    }
}

pub type FlockStrategyKind = std::any::TypeId;

#[derive(Debug, Clone, Copy, Component)]
pub struct FlockStrategy {
    pub flock_entt: Entity,
    pub kind: FlockStrategyKind,
}

impl FlockStrategy {
    pub fn new(flock_entt: Entity, kind: FlockStrategyKind) -> Self {
        Self { flock_entt, kind }
    }

    /// Get a reference to the flock strategy's flock entt.
    pub fn flock_entt(&self) -> &Entity {
        &self.flock_entt
    }

    /// Get a reference to the flock strategy's kind.
    pub fn kind(&self) -> FlockStrategyKind {
        self.kind
    }
}
