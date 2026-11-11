struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex fn vs(
    @builtin(vertex_index) i: u32,
    @location(0) dest: vec4<f32>,
    @location(1) uv0: vec2<f32>,
    @location(2) uv1: vec2<f32>,
    @location(3) uv2: vec2<f32>,
    @location(4) uv3: vec2<f32>,
) -> VertexOut {
    let corners = array<vec2<f32>,4>(vec2(0.,0.), vec2(1.,0.), vec2(0.,1.), vec2(1.,1.));
    let uvs = array<vec2<f32>,4>(uv0,uv1,uv2,uv3);
    var out: VertexOut;
    out.position = vec4(dest.xy + corners[i] * dest.zw, 0., 1.);
    out.uv = uvs[i];
    return out;
}
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var nearest: sampler;
@fragment fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(image, nearest, in.uv);
}
