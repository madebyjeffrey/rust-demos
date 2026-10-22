use bevy::prelude::*;
use std::fmt;

/// An axis coordinate expressed against the owning frame or another UI node.
/// Fractions are relative to the node's resolved border box; 0 is start, 1 is end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisAnchor {
    Frame(f32),
    Node(Entity, f32),
    Between((Entity, f32), (Entity, f32)),
}

/// Independently anchored x/y coordinates plus a logical-pixel offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub x: AxisAnchor,
    pub y: AxisAnchor,
    pub offset: Vec2,
}

impl Anchor {
    pub fn at(x: AxisAnchor, y: AxisAnchor) -> Self {
        Self {
            x,
            y,
            offset: Vec2::ZERO,
        }
    }
    pub fn frame(x: f32, y: f32) -> Self {
        Self::at(AxisAnchor::Frame(x), AxisAnchor::Frame(y))
    }
    pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = Vec2::new(x, y);
        self
    }
}

/// An orthogonal route with uniform logical width, round caps, and circular bends.
#[derive(Clone, Debug, PartialEq)]
pub struct Rail {
    pub points: Vec<Anchor>,
    pub width: f32,
    pub radius: f32,
    pub color: Color,
}

impl Rail {
    pub fn new(points: impl IntoIterator<Item = Anchor>, width: f32, color: Color) -> Self {
        Self {
            points: points.into_iter().collect(),
            width,
            radius: 0.,
            color,
        }
    }
    pub fn rounded(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

/// One owner composites the union of all rails exactly once, including translucent junctions.
/// Reserve corridors with normal Bevy layout; this paint never changes child allocation.
/// Do not also attach a different `MaterialNode` to the owning entity.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SharedFrame {
    pub rails: Vec<Rail>,
    pub opacity: f32,
    pub guides: bool,
}

impl SharedFrame {
    pub fn new(rails: impl IntoIterator<Item = Rail>) -> Self {
        Self {
            rails: rails.into_iter().collect(),
            opacity: 1.,
            guides: false,
        }
    }

    /// Resolve logical geometry independently of a renderer. `node_rect` returns
    /// border boxes in this frame's local logical coordinates. The plugin supplies
    /// this adapter from Bevy's post-layout transforms; tests use the same interface.
    pub fn resolve(
        &self,
        size: Vec2,
        mut node_rect: impl FnMut(Entity) -> Option<Rect>,
    ) -> Result<FrameGeometry, FrameError> {
        if !size.is_finite()
            || size.min_element() < 0.
            || !self.opacity.is_finite()
            || !(0. ..=1.).contains(&self.opacity)
        {
            return Err(FrameError::InvalidNumber);
        }
        let mut segments = Vec::new();
        for rail in &self.rails {
            if !rail.width.is_finite()
                || rail.width < 0.
                || !rail.radius.is_finite()
                || rail.radius < 0.
                || !rail
                    .color
                    .to_linear()
                    .to_f32_array()
                    .iter()
                    .all(|v| v.is_finite())
            {
                return Err(FrameError::InvalidNumber);
            }
            if rail.points.len() < 2 {
                return Err(FrameError::TooFewPoints);
            }
            let mut points = Vec::new();
            for anchor in &rail.points {
                let point = Vec2::new(
                    resolve_axis(anchor.x, 0, size, &mut node_rect)?,
                    resolve_axis(anchor.y, 1, size, &mut node_rect)?,
                ) + anchor.offset;
                if !point.is_finite() {
                    return Err(FrameError::InvalidNumber);
                }
                if points
                    .last()
                    .is_none_or(|last: &Vec2| last.distance(point) > 0.001)
                {
                    points.push(point);
                }
            }
            // Remove redundant collinear points. Reversals and diagonal routes are errors.
            let mut clean: Vec<Vec2> = Vec::new();
            for point in points {
                while clean.len() >= 2 {
                    let n = clean.len();
                    let incoming = (clean[n - 1] - clean[n - 2]).normalize();
                    let outgoing = (point - clean[n - 1]).normalize();
                    if incoming.dot(outgoing) > 0.999 {
                        clean.pop();
                    } else {
                        break;
                    }
                }
                clean.push(point);
            }
            if clean.len() < 2 || rail.width == 0. {
                continue;
            }
            let n = clean.len();
            let mut radii = vec![0.; n];
            for pair in clean.windows(2) {
                let delta = pair[1] - pair[0];
                if delta.x.abs().min(delta.y.abs()) > 0.001 {
                    return Err(FrameError::NonOrthogonal);
                }
            }
            for i in 1..n - 1 {
                let before = clean[i] - clean[i - 1];
                let after = clean[i + 1] - clean[i];
                if before.normalize().dot(after.normalize()).abs() > 0.001 {
                    return Err(FrameError::NonOrthogonal);
                }
                let radius = rail
                    .radius
                    .min(before.length() / 2.)
                    .min(after.length() / 2.);
                if radius > 0. && radius < rail.width / 2. {
                    return Err(FrameError::RadiusTooTight);
                }
                radii[i] = radius;
            }
            let mut cursor = clean[0];
            for i in 1..n - 1 {
                let r = radii[i];
                let incoming = (clean[i] - clean[i - 1]).normalize();
                let outgoing = (clean[i + 1] - clean[i]).normalize();
                let start = clean[i] - incoming * r;
                let end = clean[i] + outgoing * r;
                segments.push(Segment::line(cursor, start, rail));
                if r > 0. {
                    let center = start + outgoing * r;
                    segments.push(Segment {
                        geometry: center.extend(r).extend(0.),
                        metadata: Vec4::new(
                            1.,
                            rail.width,
                            (incoming - outgoing).x,
                            (incoming - outgoing).y,
                        ),
                        color: rail.color.to_linear().to_vec4(),
                    });
                }
                cursor = end;
            }
            segments.push(Segment::line(cursor, clean[n - 1], rail));
            if segments.len() > MAX_SEGMENTS {
                return Err(FrameError::TooComplex);
            }
        }
        Ok(FrameGeometry { segments })
    }
}

fn resolve_axis(
    anchor: AxisAnchor,
    axis: usize,
    size: Vec2,
    node_rect: &mut impl FnMut(Entity) -> Option<Rect>,
) -> Result<f32, FrameError> {
    let mut edge = |entity, fraction| {
        let rect = node_rect(entity).ok_or(FrameError::MissingNode(entity))?;
        Ok::<_, FrameError>(rect.min[axis] + rect.size()[axis] * fraction)
    };
    match anchor {
        AxisAnchor::Frame(fraction) => Ok(size[axis] * fraction),
        AxisAnchor::Node(entity, fraction) => edge(entity, fraction),
        AxisAnchor::Between((a, fa), (b, fb)) => Ok((edge(a, fa)? + edge(b, fb)?) / 2.),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    InvalidNumber,
    TooFewPoints,
    NonOrthogonal,
    RadiusTooTight,
    TooComplex,
    MissingNode(Entity),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for FrameError {}

/// Resolved union geometry. Geometric queries do not install a picking backend.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameGeometry {
    pub(crate) segments: Vec<Segment>,
}

impl FrameGeometry {
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
    pub fn contains(&self, point: Vec2) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.distance(point) <= 0.)
    }
}

pub(crate) const MAX_SEGMENTS: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, bevy::render::render_resource::ShaderType)]
pub(crate) struct Segment {
    pub geometry: Vec4,
    pub metadata: Vec4,
    pub color: Vec4,
}

impl Segment {
    fn line(a: Vec2, b: Vec2, rail: &Rail) -> Self {
        Self {
            geometry: Vec4::new(a.x, a.y, b.x, b.y),
            metadata: Vec4::new(0., rail.width, 0., 0.),
            color: rail.color.to_linear().to_vec4(),
        }
    }
    fn distance(&self, point: Vec2) -> f32 {
        let g = self.geometry;
        let distance = if self.metadata.x == 0. {
            let a = Vec2::new(g.x, g.y);
            let ab = Vec2::new(g.z, g.w) - a;
            let t = ((point - a).dot(ab) / ab.length_squared().max(0.0001)).clamp(0., 1.);
            (point - a - t * ab).length()
        } else {
            let q = point - Vec2::new(g.x, g.y);
            let sign = Vec2::new(self.metadata.z, self.metadata.w);
            if q.x * sign.x >= 0. && q.y * sign.y >= 0. {
                (q.length() - g.z).abs()
            } else {
                q.distance(Vec2::new(sign.x * g.z, 0.))
                    .min(q.distance(Vec2::new(0., sign.y * g.z)))
            }
        };
        distance - self.metadata.y / 2.
    }
}
