struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) bounds: vec4<f32>,
}
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@vertex fn vertex(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) bounds: vec4<f32>,
) -> Vertex {
    return Vertex(vec4<f32>(position, 0.0, 1.0), uv, color, bounds);
}
@fragment fn fragment(input: Vertex) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, clamp(input.uv, input.bounds.xy, input.bounds.zw)) * input.color;
}
