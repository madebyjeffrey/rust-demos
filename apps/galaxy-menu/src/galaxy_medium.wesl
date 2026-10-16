struct DustSettings {
    world_to_galaxy: mat4x4<f32>,
    bounds_and_density: vec4<f32>,
    absorption: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> dust_settings: DustSettings;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var dust_volume: texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var dust_sampler: sampler;

fn volume_interval(origin: vec3<f32>, direction: vec3<f32>, bounds: vec3<f32>) -> vec2<f32> {
    let signs = select(vec3<f32>(-1.0), vec3<f32>(1.0), direction >= vec3<f32>(0.0));
    let reciprocal = signs / max(abs(direction), vec3<f32>(1e-6));
    let first = (-bounds - origin) * reciprocal;
    let last = (bounds - origin) * reciprocal;
    let near = min(first, last);
    let far = max(first, last);
    return vec2<f32>(max(max(near.x, max(near.y, near.z)), 0.0), min(far.x, min(far.y, far.z)));
}

fn dust_density(point: vec3<f32>) -> f32 {
    let uvw = point / (dust_settings.bounds_and_density.xyz * 2.0) + vec3<f32>(0.5);
    return textureSampleLevel(dust_volume, dust_sampler, uvw, 0.0).r;
}

fn starlight_transmittance(camera_world: vec3<f32>, star_world: vec3<f32>) -> vec3<f32> {
    let origin = (dust_settings.world_to_galaxy * vec4<f32>(camera_world, 1.0)).xyz;
    let star = (dust_settings.world_to_galaxy * vec4<f32>(star_world, 1.0)).xyz;
    let distance = length(star - origin);
    let direction = (star - origin) / max(distance, 0.0001);
    // Clip stellar extinction to the thin dust layer, independently of the wider luminous halo.
    let bounds = dust_settings.bounds_and_density.xyz;
    let dust_bounds = vec3<f32>(bounds.x, dust_settings.absorption.w, bounds.z);
    let interval = volume_interval(origin, direction, dust_bounds);
    // Stop at the star, preserving the light of stars in front of the dust lane.
    let exit = min(interval.y, distance);
    if exit <= interval.x { return vec3<f32>(1.0); }
    let step_length = (exit - interval.x) / 16.0;
    var column_density = 0.0;
    for (var i = 0u; i < 16u; i += 1u) {
        let point = origin + direction * (interval.x + (f32(i) + 0.5) * step_length);
        column_density += dust_density(point) * step_length;
    }
    return exp(-column_density * dust_settings.absorption.rgb);
}
