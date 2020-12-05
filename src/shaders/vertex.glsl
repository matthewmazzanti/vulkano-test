#version 450

// Triangle vertex positions
layout(location = 0) in vec2 pos;

// Instance data
layout(location = 1) in vec2 pos_offset;
layout(location = 2) in float angle;
layout(location = 3) in float scale;

mat2 rotation(in float angle) {
    return mat2(
        cos(angle), -sin(angle),
        sin(angle),  cos(angle)
    );
}

void main() {
    vec2 stretch = vec2(1.0, 1920.0/1080.0);
    vec2 vertex = rotation(radians(angle)) * pos * scale * stretch + pos_offset;
    gl_Position = vec4(vertex, 0.0, 1.0);
}
