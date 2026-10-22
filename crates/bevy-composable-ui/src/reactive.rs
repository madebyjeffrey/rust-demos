//! Resource lenses, declarative subscriptions, and sibling-keyed reconciliation.

use crate::{FrameRegionsPlugin, views::View};
use bevy::{input_focus::InputFocus, prelude::*, ui::UiSystems, ui_widgets::Activate};
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
};

type Reader<T> = Arc<dyn Fn(&World) -> Result<T, BindingError> + Send + Sync>;
type Writer<T> = Arc<dyn Fn(&mut World, T) -> Result<(), BindingError> + Send + Sync>;
type Action = Arc<dyn Fn(&mut World) + Send + Sync>;
type Snapshot = Arc<dyn Any + Send + Sync>;
type Initializer = Box<dyn FnOnce(&mut World, Entity) + Send + Sync>;

/// A typed resource lens. `read` creates a read-only selector; `resource` also writes through.
pub struct Binding<T> {
    read: Reader<T>,
    write: Option<Writer<T>>,
}
impl<T> Clone for Binding<T> {
    fn clone(&self) -> Self {
        Self {
            read: self.read.clone(),
            write: self.write.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingError {
    MissingResource(&'static str),
    ReadOnly,
}
impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BindingError {}

impl<T: Clone + PartialEq + Send + Sync + 'static> Binding<T> {
    pub fn read<R: Resource>(select: impl Fn(&R) -> T + Send + Sync + 'static) -> Self {
        Self {
            read: Arc::new(move |world| {
                world
                    .get_resource::<R>()
                    .map(&select)
                    .ok_or(BindingError::MissingResource(std::any::type_name::<R>()))
            }),
            write: None,
        }
    }
    pub fn resource<R: Resource<Mutability = bevy::ecs::component::Mutable>>(
        select: impl Fn(&R) -> T + Send + Sync + 'static,
        write: impl Fn(&mut R, T) + Send + Sync + 'static,
    ) -> Self {
        let mut binding = Self::read(select);
        binding.write = Some(Arc::new(move |world, value| {
            let mut resource = world
                .get_resource_mut::<R>()
                .ok_or(BindingError::MissingResource(std::any::type_name::<R>()))?;
            write(&mut resource, value);
            Ok(())
        }));
        binding
    }
    pub fn get(&self, world: &World) -> Result<T, BindingError> {
        (self.read)(world)
    }
    pub fn set(&self, world: &mut World, value: T) -> Result<(), BindingError> {
        let write = self.write.as_ref().ok_or(BindingError::ReadOnly)?;
        if self.get(world)? != value {
            write(world, value)?;
        }
        Ok(())
    }
    pub fn update(
        &self,
        world: &mut World,
        update: impl FnOnce(&mut T),
    ) -> Result<(), BindingError> {
        let mut value = self.get(world)?;
        update(&mut value);
        self.set(world, value)
    }
    /// Projection is read-only: an inverse mapping is not inferred.
    pub fn map<U: Clone + PartialEq + Send + Sync + 'static>(
        self,
        project: impl Fn(&T) -> U + Send + Sync + 'static,
    ) -> Binding<U> {
        Binding {
            read: Arc::new(move |world| self.get(world).map(|value| project(&value))),
            write: None,
        }
    }
    /// The selected value is compared each update. Rendering only runs when it changes.
    pub fn view<V: View + 'static>(
        self,
        render: impl Fn(&T) -> V + Send + Sync + 'static,
    ) -> BoundView {
        BoundView {
            node: Node {
                flex_direction: FlexDirection::Column,
                min_width: px(0),
                min_height: px(0),
                ..default()
            },
            source: Arc::new(TypedSource {
                binding: self,
                render: move |value: &T| render(value).into_node(),
            }),
            kind: TypeId::of::<T>(),
        }
    }
}

/// A stable transparent host for a reactive subtree. Its child is reconciled in place.
pub struct BoundView {
    pub node: Node,
    source: Arc<dyn Source>,
    kind: TypeId,
}
impl BoundView {
    pub fn grow(mut self, weight: f32) -> Self {
        self.node.flex_grow = weight;
        self.node.flex_basis = px(0);
        self
    }
}
impl View for BoundView {
    fn into_node(self) -> ViewNode {
        let mut node = ViewNode::element(self.node, vec![]);
        node.kind = Kind::Bound(self.kind);
        node.source = Some(self.source);
        node
    }
}

trait Source: Send + Sync {
    fn prepare(
        &self,
        world: &World,
        previous: Option<&Snapshot>,
    ) -> Result<Option<(Snapshot, ViewNode)>, BindingError>;
}
struct TypedSource<T, F> {
    binding: Binding<T>,
    render: F,
}
impl<T: Clone + PartialEq + Send + Sync + 'static, F: Fn(&T) -> ViewNode + Send + Sync> Source
    for TypedSource<T, F>
{
    fn prepare(
        &self,
        world: &World,
        previous: Option<&Snapshot>,
    ) -> Result<Option<(Snapshot, ViewNode)>, BindingError> {
        let value = self.binding.get(world)?;
        if previous
            .and_then(|old| old.downcast_ref::<T>())
            .is_some_and(|old| *old == value)
        {
            return Ok(None);
        }
        let node = (self.render)(&value);
        Ok(Some((Arc::new(value), node)))
    }
}

/// Keys are scoped to siblings. Text and numeric keys occupy separate namespaces.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Text(String),
    Number(u64),
}
impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}
impl From<String> for Key {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
impl From<u64> for Key {
    fn from(value: u64) -> Self {
        Self::Number(value)
    }
}
impl From<u32> for Key {
    fn from(value: u32) -> Self {
        Self::Number(value.into())
    }
}
impl From<usize> for Key {
    fn from(value: usize) -> Self {
        Self::Number(value as u64)
    }
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ViewKey(pub Key);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewError {
    DuplicateKey(Key),
    TooDeep,
    NotMounted(Entity),
    Binding(BindingError),
}
impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ViewError {}
#[derive(Component, Default, Debug)]
pub struct ViewDiagnostic(pub Option<ViewError>);

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    Placeholder,
    Element,
    Text,
    Bound(TypeId),
}

/// An owned, type-erased description; useful for heterogeneous branches and collections.
pub struct ViewNode {
    pub(crate) kind: Kind,
    pub(crate) key: Option<Key>,
    pub(crate) tags: Vec<TypeId>,
    pub(crate) node: Option<Node>,
    pub(crate) text: Option<(String, f32, Color)>,
    pub(crate) paint: Option<(Color, Color)>,
    pub(crate) initials: Vec<Initializer>,
    pub(crate) activation: Option<Action>,
    source: Option<Arc<dyn Source>>,
    children: Vec<ViewNode>,
}
impl ViewNode {
    pub(crate) fn element(node: Node, children: Vec<ViewNode>) -> Self {
        Self {
            kind: Kind::Element,
            key: None,
            tags: vec![],
            node: Some(node),
            text: None,
            paint: None,
            initials: vec![],
            activation: None,
            source: None,
            children,
        }
    }
}
impl View for ViewNode {
    fn into_node(self) -> Self {
        self
    }
}

#[derive(Component, Clone)]
struct Mounted {
    kind: Kind,
    tags: Vec<TypeId>,
    key: Option<Key>,
    children: Vec<Entity>,
}
#[derive(Component)]
struct ReactiveRoot {
    source: Arc<dyn Source>,
    selected: Option<Snapshot>,
}
#[derive(Component)]
struct Activation(Action);

pub struct ComposableUiPlugin;
impl Plugin for ComposableUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ReactiveViewsPlugin, FrameRegionsPlugin));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReactiveViewsSystems;
pub struct ReactiveViewsPlugin;
impl Plugin for ReactiveViewsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputFocus>()
            .add_observer(activate)
            .add_systems(
                PostUpdate,
                update_bindings
                    .before(UiSystems::Prepare)
                    .in_set(ReactiveViewsSystems),
            );
    }
}

fn activate(event: On<Activate>, actions: Query<&Activation>, mut commands: Commands) {
    if let Ok(action) = actions.get(event.entity) {
        let action = action.0.clone();
        commands.queue(move |world: &mut World| action(world));
    }
}

pub(crate) fn mount(node: ViewNode, commands: &mut Commands) -> Entity {
    let entity = commands.spawn_empty().id();
    commands.queue(move |world: &mut World| {
        if let Err(error) = validate(&node, 0) {
            world.entity_mut(entity).insert((
                Node::default(),
                Pickable::IGNORE,
                ViewDiagnostic(Some(error)),
                Mounted {
                    kind: Kind::Placeholder,
                    tags: vec![],
                    key: None,
                    children: vec![],
                },
            ));
        } else {
            mount_into(world, entity, node, 0);
        }
    });
    entity
}

/// Reconcile a mounted view. Duplicate sibling keys are rejected before modifying the tree.
/// Returns the surviving root, which can change when its kind or key changes.
pub fn reconcile(world: &mut World, root: Entity, view: impl View) -> Result<Entity, ViewError> {
    let node = view.into_node();
    let depth = mounted_depth(world, root);
    validate(&node, depth)?;
    if world.get::<Mounted>(root).is_none() {
        return Err(ViewError::NotMounted(root));
    }
    let parent = world.get::<ChildOf>(root).map(ChildOf::parent);
    let siblings = parent
        .and_then(|p| world.get::<Children>(p))
        .map(|c| c.iter().collect::<Vec<_>>());
    let next = reconcile_node(world, root, node, depth);
    if next != root
        && let (Some(parent), Some(mut children)) = (parent, siblings)
    {
        for child in &mut children {
            if *child == root {
                *child = next;
            }
        }
        world.entity_mut(parent).replace_children(&children);
        if let Some(mut mounted) = world.get_mut::<Mounted>(parent) {
            for child in &mut mounted.children {
                if *child == root {
                    *child = next;
                }
            }
        }
    }
    Ok(next)
}

fn validate(node: &ViewNode, depth: usize) -> Result<(), ViewError> {
    if depth >= 64 {
        return Err(ViewError::TooDeep);
    }
    let mut keys = HashSet::new();
    for child in &node.children {
        if let Some(key) = &child.key
            && !keys.insert(key)
        {
            return Err(ViewError::DuplicateKey(key.clone()));
        }
        validate(child, depth + 1)?;
    }
    Ok(())
}

fn put<C: Component<Mutability = bevy::ecs::component::Mutable> + PartialEq>(
    world: &mut World,
    entity: Entity,
    value: C,
) {
    if world.get::<C>(entity) != Some(&value) {
        world.entity_mut(entity).insert(value);
    }
}

fn properties(world: &mut World, entity: Entity, node: &mut ViewNode) {
    if let Some(value) = node.node.take() {
        put(world, entity, value);
    }
    if let Some((value, size, color)) = node.text.take() {
        put(world, entity, Text::new(value));
        let size = FontSize::Px(size);
        if let Some(mut font) = world.get_mut::<TextFont>(entity)
            && font.font_size != size
        {
            font.font_size = size;
        }
        put(world, entity, TextColor(color));
    }
    if let Some((fill, border)) = node.paint.take() {
        put(world, entity, BackgroundColor(fill));
        put(world, entity, BorderColor::all(border));
    }
    if let Some(action) = node.activation.take() {
        world.entity_mut(entity).insert(Activation(action));
    } else {
        world.entity_mut(entity).remove::<Activation>();
    }
}

fn mount_into(world: &mut World, entity: Entity, mut node: ViewNode, depth: usize) {
    properties(world, entity, &mut node);
    world.entity_mut(entity).insert(Pickable::IGNORE);
    for initialize in node.initials.drain(..) {
        initialize(world, entity);
    }
    if let Some(key) = &node.key {
        world.entity_mut(entity).insert(ViewKey(key.clone()));
    }
    world.entity_mut(entity).insert(Mounted {
        kind: node.kind,
        tags: node.tags,
        key: node.key,
        children: vec![],
    });
    if let Some(source) = node.source {
        world.entity_mut(entity).insert((
            ReactiveRoot {
                source,
                selected: None,
            },
            ViewDiagnostic::default(),
        ));
        refresh_binding(world, entity, depth + 1);
    } else {
        reconcile_children(world, entity, node.children, depth + 1);
    }
}

fn reconcile_node(world: &mut World, entity: Entity, mut node: ViewNode, depth: usize) -> Entity {
    let compatible = world
        .get::<Mounted>(entity)
        .is_some_and(|old| old.kind == node.kind && old.tags == node.tags && old.key == node.key);
    if !compatible {
        let next = world.spawn_empty().id();
        mount_into(world, next, node, depth);
        unmount(world, entity);
        return next;
    }
    properties(world, entity, &mut node);
    // Tags are mount-only: user state, native hover/press, and scroll components survive.
    if let Some(source) = node.source {
        world.entity_mut(entity).insert(ReactiveRoot {
            source,
            selected: None,
        });
        refresh_binding(world, entity, depth + 1);
    } else {
        reconcile_children(world, entity, node.children, depth + 1);
    }
    entity
}

fn reconcile_children(world: &mut World, parent: Entity, children: Vec<ViewNode>, depth: usize) {
    let old = world.get::<Mounted>(parent).unwrap().children.clone();
    let keyed: HashMap<_, _> = old
        .iter()
        .filter_map(|e| {
            world
                .get::<Mounted>(*e)
                .and_then(|m| m.key.clone().map(|k| (k, *e)))
        })
        .collect();
    let mut next = Vec::new();
    for (index, child) in children.into_iter().enumerate() {
        let matching = if let Some(key) = &child.key {
            keyed.get(key).copied()
        } else {
            old.get(index)
                .copied()
                .filter(|e| world.get::<Mounted>(*e).is_some_and(|m| m.key.is_none()))
        };
        let entity = if let Some(entity) = matching {
            reconcile_node(world, entity, child, depth)
        } else {
            let entity = world.spawn_empty().id();
            mount_into(world, entity, child, depth);
            entity
        };
        next.push(entity);
    }
    for entity in old {
        if !next.contains(&entity) {
            unmount(world, entity);
        }
    }
    world.entity_mut(parent).replace_children(&next);
    world.get_mut::<Mounted>(parent).unwrap().children = next;
}

fn unmount(world: &mut World, entity: Entity) {
    if world.get_entity(entity).is_err() {
        return;
    }
    let focus = world.get_resource::<InputFocus>().and_then(InputFocus::get);
    let mut cursor = focus;
    while let Some(current) = cursor {
        if current == entity {
            world.resource_mut::<InputFocus>().clear();
            break;
        }
        cursor = world.get::<ChildOf>(current).map(ChildOf::parent);
    }
    world.despawn(entity);
}

fn refresh_binding(world: &mut World, entity: Entity, depth: usize) {
    let Some(root) = world.get::<ReactiveRoot>(entity) else {
        return;
    };
    let source = root.source.clone();
    let previous = root.selected.clone();
    let result = if depth >= 64 {
        Err(ViewError::TooDeep)
    } else {
        source
            .prepare(world, previous.as_ref())
            .map_err(ViewError::Binding)
    };
    let error = match result {
        Ok(Some((selected, node))) => {
            let result = validate(&node, depth);
            // Cache rejected selections too, avoiding a repeated failing render every frame.
            world.get_mut::<ReactiveRoot>(entity).unwrap().selected = Some(selected);
            match result {
                Ok(()) => {
                    reconcile_children(world, entity, vec![node], depth);
                    None
                }
                Err(error) => Some(error),
            }
        }
        Ok(None) => return,
        Err(error) => {
            // Resource recovery must revalidate even an equal previously rejected selection.
            if matches!(error, ViewError::Binding(_)) {
                world.get_mut::<ReactiveRoot>(entity).unwrap().selected = None;
            }
            Some(error)
        }
    };
    if world
        .get::<ViewDiagnostic>(entity)
        .is_none_or(|old| old.0 != error)
    {
        if let Some(error) = &error {
            warn!(%error, "Reactive view update rejected; retaining last valid subtree");
        }
        world.entity_mut(entity).insert(ViewDiagnostic(error));
    }
}

fn update_bindings(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<ReactiveRoot>>()
        .iter(world)
        .collect();
    for entity in entities {
        let depth = mounted_depth(world, entity) + 1;
        refresh_binding(world, entity, depth);
    }
}

fn mounted_depth(world: &World, entity: Entity) -> usize {
    let mut depth = 0;
    let mut parent = world.get::<ChildOf>(entity).map(ChildOf::parent);
    while let Some(entity) = parent {
        if world.get::<Mounted>(entity).is_none() {
            break;
        }
        depth += 1;
        parent = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    depth
}
