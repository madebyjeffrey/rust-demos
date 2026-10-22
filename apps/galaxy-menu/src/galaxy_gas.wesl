import bevy_pbr::{
    render::{mesh_functions, mesh_view_bindings::view},
    prepass::utils::prepass_depth,
};
import package::galaxy_medium::{dust_settings, dust_density, volume_interval};

@group(constants::MATERIAL_BIND_GROUP) @binding(3) var gas_volume: texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var gas_sampler: sampler;

struct GasVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
}

struct GasFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) local_position: vec3<f32>,
    @location(1) @interpolate(flat) local_camera: vec3<f32>,
    @location(2) @interpolate(flat) instance_index: u32,
}

@vertex
fn vertex(input: GasVertex) -> GasFragment {
    let world_from_local = mesh_functions::get_world_from_local(input.instance_index);
    let local_from_world = mesh_functions::get_local_from_world(input.instance_index);
    var out: GasFragment;
    out.position = view.clip_from_world * world_from_local * vec4<f32>(input.position, 1.0);
    out.local_position = input.position;
    out.local_camera = (local_from_world * vec4<f32>(view.world_position, 1.0)).xyz;
    out.instance_index = input.instance_index;
    return out;
}

@fragment
fn fragment(input: GasFragment) -> @location(0) vec4<f32> {
    let origin = input.local_camera;
    let direction = normalize(input.local_position - origin);
    let bounds = dust_settings.bounds_and_density.xyz;
    let interval = volume_interval(origin, direction, bounds);
    let entry = interval.x;
    var exit = interval.y;

    @if(DEPTH_PREPASS) {
        // Respect the scene depth when compositing the galaxy's diffuse light.
        let depth = prepass_depth(input.position, 0u);
        if depth > 0.0 {
            let uv = (input.position.xy - view.viewport.xy) / view.viewport.zw;
            let ndc = vec3<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), depth);
            let world = view.world_from_clip * vec4<f32>(ndc, 1.0);
            let local_from_world = mesh_functions::get_local_from_world(input.instance_index);
            let surface = (local_from_world * vec4<f32>(world.xyz / world.w, 1.0)).xyz;
            exit = min(exit, dot(surface - origin, direction));
        }
    }

    if exit <= entry { return vec4<f32>(0.0); }
    let step_length = (exit - entry) / 64.0;
    var color = vec3<f32>(0.0);
    var transmittance = vec3<f32>(1.0);
    // Fixed midpoint samples and trilinear filtering keep motion stable without pixel noise.
    for (var i = 0u; i < 64u; i += 1u) {
        let point = origin + direction * (entry + (f32(i) + 0.5) * step_length);
        let uvw = point / (bounds * 2.0) + vec3<f32>(0.5);
        let gas = textureSampleLevel(gas_volume, gas_sampler, uvw, 0.0);
        let gas_opacity = gas.a * gas.a;
        let opacity = 1.0 - pow(max(1.0 - gas_opacity, 0.0001), step_length * dust_settings.bounds_and_density.w);
        let transmission = exp(-dust_density(point) * step_length * dust_settings.absorption.rgb);
        color += transmittance * sqrt(transmission) * opacity * gas.rgb * 2.8;
        transmittance *= (1.0 - opacity) * transmission;
        if max(transmittance.r, max(transmittance.g, transmittance.b)) < 0.005 { break; }
    }
    // Stellar light has already been attenuated only by the dust in front of each star.
    return vec4<f32>(color, 0.0);
}
