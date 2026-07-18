// Frosted-glass fragment shader for UI nodes (`GlassMaterial`).
//
// Kept deliberately tiny: a rounded-rect mask (crisp or feathered),
// a blurred backdrop sample (frost), a thin border rim, and a cursor
// glow on hover.

#import bevy_ui::ui_vertex_output::UiVertexOutput

// Base glass tint; alpha is the body opacity.
@group(1) @binding(0)
var<uniform> tint: vec4<f32>;
// x: rim brightness, y: frost blur radius (physical px),
// z: rim opacity, w: cursor glow strength.
@group(1) @binding(1)
var<uniform> params: vec4<f32>;
// xy: cursor position (physical px), z: glow radius, w: radius scale.
@group(1) @binding(2)
var<uniform> glow: vec4<f32>;
// Backdrop rect in physical px (min.xy, size.zw); zero disables frost.
@group(1) @binding(3)
var<uniform> backdrop_rect: vec4<f32>;
@group(1) @binding(4)
var backdrop_tex: texture_2d<f32>;
@group(1) @binding(5)
var backdrop_smp: sampler;
// x: edge feather in px (0 = crisp).
@group(1) @binding(6)
var<uniform> extra: vec4<f32>;

// Signed distance to a rounded box; `r4` corner order: TL, TR, BR, BL.
fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, r4: vec4<f32>) -> f32 {
    var r = r4.x;
    if p.x > 0.0 && p.y < 0.0 { r = r4.y; }
    if p.x > 0.0 && p.y > 0.0 { r = r4.z; }
    if p.x < 0.0 && p.y > 0.0 { r = r4.w; }
    let q = abs(p) - half_size + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * in.size;
    let d = sd_rounded_box(p, in.size * 0.5, in.border_radius);

    // Masks: pane shape (crisp or feathered) and the thin border rim.
    let shape = 1.0 - smoothstep(-max(extra.x, 1.0), 0.5, d);
    let rim = 1.0 - smoothstep(0.0, 3.0, -d);

    // Cursor glow (hover only; strength is gated CPU-side).
    let gd = distance(in.position.xy, glow.xy);
    let gr = glow.z * glow.w;
    let cursor = exp(-(gd * gd) / max(gr * gr, 1.0)) * params.w;

    // Frost: 3x3 blur of the backdrop, mixed under the tint.
    var col = tint.rgb;
    var alpha = tint.a;
    if tint.a > 0.001 && params.y > 0.0 && backdrop_rect.z > 0.5 {
        var acc = vec3<f32>(0.0);
        for (var x = -1; x <= 1; x++) {
            for (var y = -1; y <= 1; y++) {
                let pos = in.position.xy
                    + vec2<f32>(f32(x), f32(y)) * params.y;
                let uv = clamp(
                    (pos - backdrop_rect.xy) / backdrop_rect.zw,
                    vec2<f32>(0.0),
                    vec2<f32>(1.0),
                );
                acc += textureSampleLevel(
                    backdrop_tex,
                    backdrop_smp,
                    uv,
                    0.0,
                )
                .rgb;
            }
        }
        col = mix(acc / 9.0, tint.rgb, tint.a);
        alpha = max(tint.a, 0.88);
    }

    col += vec3<f32>(rim * (params.x + cursor * 0.8) + cursor * 0.12);
    alpha = max(alpha + cursor * 0.06, rim * params.z);

    return vec4<f32>(col, alpha * shape);
}
