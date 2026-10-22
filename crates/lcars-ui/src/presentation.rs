use crate::{ControlStyle, INK, MUTED, PANEL, Progress, ProgressFill, SelectionMark, WHITE};
use bevy::{
    ecs::query::QueryData,
    input_focus::{IsFocused, IsFocusedHelper},
    picking::hover::Hovered,
    prelude::*,
    ui::{Checked, InteractionDisabled, Pressed},
    ui_widgets::Checkbox,
};

#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct Appearance {
    entity: Entity,
    style: &'static ControlStyle,
    hovered: &'static Hovered,
    pressed: Has<Pressed>,
    checked: Has<Checked>,
    checkbox: Has<Checkbox>,
    disabled: Has<InteractionDisabled>,
    background: &'static mut BackgroundColor,
    border: &'static mut BorderColor,
    children: &'static Children,
    parent: &'static ChildOf,
}
pub(crate) fn style_widgets(
    focus: IsFocusedHelper,
    mut controls: Query<Appearance>,
    mut texts: Query<(&mut Text, &mut TextColor, Has<SelectionMark>)>,
) {
    for mut widget in &mut controls {
        let accent = widget.style.tone.color();
        let filled = widget.checked || widget.style.primary;
        let highlighted = !widget.disabled && (filled || widget.hovered.get() || widget.pressed);
        widget.background.0 = if widget.disabled {
            PANEL
        } else if widget.pressed {
            WHITE
        } else if highlighted {
            accent
        } else {
            PANEL
        };
        let focused = focus.is_focus_visible(widget.entity)
            || (widget.checked && focus.is_focus_visible(widget.parent.parent()));
        widget.border.set_all(if widget.disabled {
            MUTED
        } else if focused {
            WHITE
        } else {
            accent
        });
        for child in widget.children.iter() {
            if let Ok((mut text, mut color, mark)) = texts.get_mut(child) {
                color.0 = if widget.disabled {
                    MUTED
                } else if highlighted {
                    INK
                } else {
                    accent
                };
                if mark {
                    text.0 = match (widget.checkbox, widget.checked) {
                        (true, true) => "[X]",
                        (true, false) => "[ ]",
                        (false, true) => "(o)",
                        (false, false) => "( )",
                    }
                    .into();
                }
            }
        }
    }
}
pub(crate) fn update_progress(
    tracks: Query<(&Progress, &Children), Changed<Progress>>,
    mut fills: Query<&mut Node, With<ProgressFill>>,
) {
    for (progress, children) in &tracks {
        let value = if progress.0.is_finite() {
            progress.0.clamp(0.0, 1.0)
        } else {
            0.0
        };
        for child in children.iter() {
            if let Ok(mut node) = fills.get_mut(child) {
                node.width = percent(value * 100.0);
            }
        }
    }
}
