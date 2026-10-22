//! Ordinary Rust values compose into retained Bevy UI entities.
//! Static values mount once. Resource-bound subtrees reconcile automatically.

use crate::reactive::{self, Key, Kind, ViewNode};
use bevy::prelude::*;
use std::{any::TypeId, sync::Arc};

pub trait View: Sized {
    fn into_node(self) -> ViewNode;

    fn mount(self, commands: &mut Commands) -> Entity {
        reactive::mount(self.into_node(), commands)
    }

    /// A sibling-scoped identity. Reordering preserves the matching entity and its local state.
    fn key(self, key: impl Into<Key>) -> ViewNode {
        let mut node = self.into_node();
        node.key = Some(key.into());
        node
    }

    /// Animate this keyed view's insertion, movement and removal.
    fn transition(self, transition: crate::ViewTransition) -> ViewNode {
        let mut node = self.into_node();
        node.transition = Some(transition);
        node
    }

    /// Connect a native `Activate` event. The current handler replaces the previous one on update.
    /// Add Bevy's headless `Button` behavior and `Pickable::default()` with `tagged`.
    fn on_activate(self, action: impl Fn(&mut World) + Send + Sync + 'static) -> ViewNode {
        let mut node = self.into_node();
        node.activation = Some(Arc::new(action));
        node
    }

    /// Initialize components once per entity lifetime; do not reapply them on reconciliation.
    fn tagged<B: Bundle>(self, bundle: B) -> Tagged<Self, B> {
        Tagged(self, bundle)
    }

    /// Padding is a wrapper: background before/after it paints different bounds.
    fn padding(self, amount: f32) -> Element<(Self,)> {
        column((self,)).pad(amount)
    }

    fn background(self, color: Color) -> Element<(Self,)> {
        column((self,)).fill(color)
    }
}

pub struct Tagged<V, B>(V, B);

impl<V: View, B: Bundle> View for Tagged<V, B> {
    fn into_node(self) -> ViewNode {
        let mut node = self.0.into_node();
        node.tags.push(TypeId::of::<B>());
        node.initials.push(Box::new(move |world, entity| {
            world.entity_mut(entity).insert(self.1);
        }));
        node
    }
}

pub trait Views {
    fn into_nodes(self) -> Vec<ViewNode>;
    fn mount_children(self, commands: &mut Commands) -> Vec<Entity>
    where
        Self: Sized,
    {
        self.into_nodes()
            .into_iter()
            .map(|node| node.mount(commands))
            .collect()
    }
}

impl Views for () {
    fn into_nodes(self) -> Vec<ViewNode> {
        Vec::new()
    }
}

macro_rules! tuple_views {
    ($($name:ident),+) => {
        impl<$($name: View),+> Views for ($($name,)+) {
            #[allow(non_snake_case)]
            fn into_nodes(self) -> Vec<ViewNode> {
                let ($($name,)+) = self;
                vec![$($name.into_node()),+]
            }
        }
    };
}

tuple_views!(A);
tuple_views!(A, B);
tuple_views!(A, B, C);
tuple_views!(A, B, C, D);
tuple_views!(A, B, C, D, E);
tuple_views!(A, B, C, D, E, F);

impl<V: View> Views for Vec<V> {
    fn into_nodes(self) -> Vec<ViewNode> {
        self.into_iter().map(|v| v.into_node()).collect()
    }
}

pub struct Element<C> {
    pub node: Node,
    children: C,
    fill: Color,
    border: Color,
}

pub fn row<C: Views>(children: C) -> Element<C> {
    Element {
        node: Node {
            flex_direction: FlexDirection::Row,
            min_width: px(0),
            min_height: px(0),
            ..default()
        },
        children,
        fill: Color::NONE,
        border: Color::NONE,
    }
}

pub fn column<C: Views>(children: C) -> Element<C> {
    let mut view = row(children);
    view.node.flex_direction = FlexDirection::Column;
    view
}

/// A CSS Grid container. Configure its tracks through [`Element::node`].
pub fn grid<C: Views>(children: C) -> Element<C> {
    let mut view = row(children);
    view.node.display = Display::Grid;
    view
}

impl<C> Element<C> {
    pub fn gap(mut self, gap: f32) -> Self {
        self.node.row_gap = px(gap);
        self.node.column_gap = px(gap);
        self
    }

    pub fn pad(mut self, padding: f32) -> Self {
        self.node.padding = UiRect::all(px(padding));
        self
    }

    pub fn grow(mut self, weight: f32) -> Self {
        self.node.flex_grow = weight;
        self.node.flex_basis = px(0);
        self
    }

    pub fn fill(mut self, color: Color) -> Self {
        self.fill = color;
        self
    }

    pub fn border(mut self, width: f32, color: Color, radius: f32) -> Self {
        self.node.border = UiRect::all(px(width));
        self.node.border_radius = BorderRadius::all(px(radius));
        self.border = color;
        self
    }
}

impl<C: Views> View for Element<C> {
    fn into_node(self) -> ViewNode {
        let mut node = ViewNode::element(self.node, self.children.into_nodes());
        node.paint = Some((self.fill, self.border));
        node
    }
}

pub struct Label {
    value: String,
    size: f32,
    color: Color,
}

pub fn text(value: impl Into<String>, size: f32, color: Color) -> Label {
    Label {
        value: value.into(),
        size,
        color,
    }
}

impl View for Label {
    fn into_node(self) -> ViewNode {
        let mut node = ViewNode::element(Node::default(), vec![]);
        node.kind = Kind::Text;
        node.node = None;
        node.text = Some((self.value, self.size, self.color));
        node
    }
}
