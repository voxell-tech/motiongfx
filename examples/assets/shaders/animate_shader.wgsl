// The time since startup data is in the globals binding which is part of the mesh_view_bindings import
#import bevy_pbr::mesh_view_bindings globals
#import bevy_pbr::mesh_vertex_output MeshVertexOutput

const EPSILON: f32 = 1e-6;
const PI: f32 = 3.14159265359;

fn rounded_shape_sdf(uv: vec2<f32>, size: vec2<f32>, sides: f32, roundness: f32) -> f32 {
    let HALF_PI: f32 = PI / 2.0;

    let inv_sides: f32 = 1.0 / sides;

    let full_angle: f32 = 2.0 * PI * inv_sides;
    let half_angle: f32 = full_angle * 0.5;
    let opposite_angle: f32 = HALF_PI - half_angle;
    var diagonal: f32 = 1.0 / cos(half_angle);

    // Chamfer values
    // Angle taken by the chamfer
    let chamfer_angle: f32 = roundness * half_angle;
    // Angle that remains
    let remaining_angle: f32 = half_angle - chamfer_angle;
    // Ratio between the length of the polygon's triangle and
    // the distance of the chamfer center to the polygon center
    let ratio: f32 = tan(remaining_angle) / tan(half_angle);

    // Center of the chamfer arc
    let chamfer_center: vec2<f32> = vec2<f32>(
        cos(half_angle),
        sin(half_angle)
    ) * ratio * diagonal;

    // Starting of the chamfer arc
    let chamfer_origin: vec2<f32> = vec2<f32>(1.0, tan(remaining_angle));

    // Using Al Kashi algebra, we determine:
    // The distance of the center of the chamfer to the center of the polygon (side A)
    let dist_a: f32 = length(chamfer_center);
    // The radius of the chamfer (side B)
    let dist_b: f32 = 1.0 - chamfer_center.x;
    // This will rescale the chamfered polygon to fit the uv space
    // diagonal = length(chamfer_center) + dist_b;

    let a_size: vec2<f32> = size * cos(PI * inv_sides);

    // Reposition and scale uv
    var a_uv: vec2<f32> = uv - 0.5;
    a_uv /= a_size;
    a_uv *= diagonal;

    var polar_uv: vec2<f32> = vec2<f32>(
        atan2(a_uv.x, a_uv.y),
        length(a_uv)
    );
    polar_uv.x += HALF_PI + 2.0 * PI;
    polar_uv.x = (polar_uv.x + half_angle) % full_angle;
    polar_uv.x = abs(polar_uv.x - half_angle);

    a_uv = vec2<f32>(cos(polar_uv.x), sin(polar_uv.x)) * polar_uv.y;

    // Calculate the angle needed for the Al Kashi algebra
    let angle_ratio: f32 = 1.0 - (polar_uv.x - remaining_angle) / chamfer_angle;
    // Calculate the distance of the polygon center to the chamfer extremity
    let dist_c: f32 = sqrt(dist_a * dist_a + dist_b * dist_b - 2.0 * dist_a * dist_b * cos(PI - half_angle * angle_ratio));

    let chamfer_zone: f32 = f32((half_angle - polar_uv.x) < chamfer_angle);
    return mix(a_uv.x, polar_uv.y / dist_c, chamfer_zone);
}

fn sdf_to_shape(sdf: f32) -> f32 {
    // Convert distance field into shape mask
    return saturate((1.0 - sdf) / fwidth(sdf));
}

@fragment
fn fragment(in: MeshVertexOutput) -> @location(0) vec4<f32> {
    let size: vec2<f32> = vec2<f32>(0.5, 0.5);
    let roundness: f32 = 0.02;

    let shape_sdf0: f32 = rounded_shape_sdf(in.uv, size, 3.0, roundness);
    let shape_sdf1: f32 = rounded_shape_sdf(in.uv, size, 4.0, roundness);

    let shape_sdf: f32 = mix(shape_sdf0, shape_sdf1, (sin(globals.time) * 0.5 + 0.5));

    let shape: f32 = sdf_to_shape(shape_sdf);

    // return vec4<f32>(in.uv, 0.0, 1.0);
    if (shape <= 0.0) {
        discard;
    }
    else {
        return vec4<f32>(shape);
    }
}
