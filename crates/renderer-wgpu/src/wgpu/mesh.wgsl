struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

fn linear(value: vec3<f32>) -> vec3<f32> {
    return select(pow((value + 0.055) / 1.055, vec3<f32>(2.4)), value / 12.92, value <= vec3<f32>(0.04045));
}

@vertex fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) column0: vec4<f32>,
    @location(3) column1: vec4<f32>,
    @location(4) column2: vec4<f32>,
    @location(5) column3: vec4<f32>,
) -> Vertex {
    let transform = mat4x4<f32>(column0, column1, column2, column3);
    return Vertex(transform * vec4<f32>(position, 1.0), vec4<f32>(linear(color.rgb), 1.0));
}

@fragment fn fragment(input: Vertex) -> @location(0) vec4<f32> {
    return input.color;
}
