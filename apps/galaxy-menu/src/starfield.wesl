import bevy_pbr::render::{mesh_functions, mesh_view_bindings::view};
import package::galaxy_medium::starlight_transmittance;

struct StarVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) center: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) corner_and_radius: vec3<f32>,
}

struct StarFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) offset: vec2<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vertex(star: StarVertex) -> StarFragment {
    let world_from_local = mesh_functions::get_world_from_local(star.instance_index);
    let center = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(star.center, 1.0));
    let clip = view.clip_from_world * center;
    let world_radius = star.corner_and_radius.z * length(world_from_local[0].xyz);
    let pixel_radius = world_radius * view.clip_from_view[1][1] * view.viewport.w * 0.5 / max(clip.w, 0.0001);

    // Convolve the star with a pixel filter. Its footprint never collapses between samples.
    let variance = pixel_radius * pixel_radius + 0.75 * 0.75;
    let sigma = sqrt(variance);
    let offset = star.corner_and_radius.xy * 3.5;
    let clip_offset = offset * sigma * 2.0 / view.viewport.zw * clip.w;

    var out: StarFragment;
    out.position = vec4<f32>(clip.xy + clip_offset, clip.zw);
    out.offset = offset;
    // Preserve integrated light: zooming out must not turn faint stars into bright dots.
    let transmission = starlight_transmittance(view.world_position, center.xyz);
    out.color = star.color.rgb * transmission * pixel_radius * pixel_radius / variance;
    return out;
}

@fragment
fn fragment(star: StarFragment) -> @location(0) vec4<f32> {
    let light = exp(-0.5 * dot(star.offset, star.offset));
    // Premultiplied blending with zero alpha adds light without hiding other stars.
    return vec4<f32>(star.color * light, 0.0);
}
