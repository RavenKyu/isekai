struct VertexOut { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32> }
@vertex fn vs(@builtin(vertex_index) i:u32)->VertexOut {
    let p=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));
    var out:VertexOut;out.position=vec4(p[i],0.,1.);out.uv=vec2((p[i].x+1.)/2.,(1.-p[i].y)/2.);return out;
}
@group(0) @binding(0) var image:texture_2d<f32>;
@group(0) @binding(1) var nearest:sampler;
@fragment fn fs(in:VertexOut)->@location(0) vec4<f32> { return textureSample(image,nearest,in.uv); }
@fragment fn fs_srgb(in:VertexOut)->@location(0) vec4<f32> {
    let c=textureSample(image,nearest,in.uv);
    let linear=select(pow((c.rgb+vec3(0.055))/vec3(1.055),vec3(2.4)),c.rgb/vec3(12.92),c.rgb<=vec3(0.04045));
    return vec4(linear,c.a);
}
