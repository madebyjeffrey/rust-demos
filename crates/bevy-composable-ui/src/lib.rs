//! Small reusable composition and shared-region layer for Bevy 0.20.
//!
//! Mount [`views::View`] values into retained entities and add [`FrameRegionsPlugin`]
//! once, or use [`ComposableUiPlugin`] for resource bindings and reconciliation too.
//! Attach a [`SharedFrame`] to any UI container to paint rounded, anchored
//! orthogonal routes. Layout reservations, resource models, and themes stay in the app.
//! Bevy remains the layout authority. Decorative frame paint ignores picking.

#![doc = include_str!("../README.md")]

mod geometry;
mod reactive;
mod render;
mod transitions;
pub mod views;

pub use geometry::{Anchor, AxisAnchor, FrameError, FrameGeometry, Rail, SharedFrame};
pub use reactive::{
    Binding, BindingError, BoundView, ComposableUiPlugin, Key, ReactiveViewsPlugin,
    ReactiveViewsSystems, ViewDiagnostic, ViewError, ViewKey, ViewNode, reconcile,
};
pub use render::{FrameDiagnostic, FrameRegionsPlugin, FrameRegionsSystems};
pub use transitions::{
    ExitingView, TransitionPhase, TransitionSettings, TransitionStatus, ViewTransition,
    ViewTransitionSystems,
};

pub mod prelude {
    pub use crate::views::{Element, Label, View, Views, column, grid, row, text};
    pub use crate::{Anchor, AxisAnchor, FrameDiagnostic, FrameRegionsPlugin, Rail, SharedFrame};
    pub use crate::{
        Binding, ComposableUiPlugin, ExitingView, ReactiveViewsPlugin, TransitionPhase,
        TransitionSettings, TransitionStatus, ViewDiagnostic, ViewKey, ViewNode, ViewTransition,
    };
}
