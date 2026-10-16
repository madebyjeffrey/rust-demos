//! Logical safe area supplied by the platform for the full-window renderer.

use bevy::prelude::*;

/// Insets in logical window coordinates. Attach to the primary `Window`.
///
/// Desktop windows default to zero. The native iOS bridge refreshes these from
/// the renderer view so rotation and multitasking use UIKit's current safe area.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowSafeArea {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[cfg(target_os = "ios")]
pub(crate) fn register_native_safe_area(app: &mut App) {
    app.add_systems(PreUpdate, refresh_native_safe_area);
}

#[cfg(target_os = "ios")]
fn refresh_native_safe_area(
    _main_thread: bevy::ecs::system::NonSendMarker,
    mut commands: Commands,
    windows: Query<(Entity, Option<&WindowSafeArea>), With<bevy::window::PrimaryWindow>>,
) {
    unsafe extern "C" {
        fn galaxy_window_safe_area(insets: *mut f32) -> bool;
    }
    let Ok((entity, previous)) = windows.single() else {
        return;
    };
    let mut insets = [0.0; 4];
    // SAFETY: NonSendMarker keeps UIKit access on the main thread. The bridge
    // writes exactly four floats and retains no pointer after returning.
    if unsafe { galaxy_window_safe_area(insets.as_mut_ptr()) } {
        let current = WindowSafeArea {
            top: insets[0],
            right: insets[1],
            bottom: insets[2],
            left: insets[3],
        };
        if previous != Some(&current) {
            commands.entity(entity).insert(current);
        }
    }
}
