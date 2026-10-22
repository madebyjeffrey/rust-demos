import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct Segment { geometry: vec4<f32>, metadata: vec4<f32>, color: vec4<f32> }
struct FrameUniform { segments: array<Segment, 32>, metrics: vec4<f32> }
@group(1) @binding(0) var<uniform> frame: FrameUniform;

fn segment_distance(p: vec2<f32>, segment: Segment) -> f32 {
    let g = segment.geometry;
    var d = 0.0;
    if segment.metadata.x < 0.5 {
        let ab = g.zw - g.xy;
        let t = clamp(dot(p - g.xy, ab) / max(dot(ab, ab), 0.0001), 0.0, 1.0);
        d = length(p - g.xy - t * ab);
    } else {
        let q = p - g.xy;
        let quadrant = segment.metadata.zw;
        if q.x * quadrant.x >= 0.0 && q.y * quadrant.y >= 0.0 { d = abs(length(q) - g.z); }
        else { d = min(length(q - vec2(quadrant.x * g.z, 0.0)), length(q - vec2(0.0, quadrant.y * g.z))); }
    }
    return d - segment.metadata.y * 0.5;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv * in.size * frame.metrics.x;
    var distance = 1e10;
    var centerline = 1e10;
    var color = vec4(0.0);
    for (var i = 0u; i < u32(frame.metrics.y); i += 1u) {
        let segment = frame.segments[i];
        let d = segment_distance(p, segment);
        centerline = min(centerline, abs(d + segment.metadata.y * 0.5));
        if d < distance { distance = d; color = segment.color; }
    }
    let aa = max(fwidth(distance), 0.35 * frame.metrics.x);
    var alpha = (1.0 - smoothstep(-aa, aa, distance)) * frame.metrics.w * color.a;
    if frame.metrics.z > 0.5 && centerline < 0.65 && fract((p.x + p.y) / 9.0) < 0.5 {
        color = vec4(0.95); alpha = 1.0;
    }
    return vec4(color.rgb, alpha);
}
