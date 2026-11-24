#version 450

layout(location = 0) in vec4 v_tex_coord;

layout(binding = 0) uniform sampler2D u_texture;

layout(location = 0) out vec4 fragColor;

void main() {
    fragColor = texture(u_texture, v_tex_coord.xy);
}
