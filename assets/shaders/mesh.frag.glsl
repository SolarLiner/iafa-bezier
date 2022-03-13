#version 330

in vec3 v_normal;
in vec2 v_uv;

uniform mat4 model;
uniform mat4 view_proj;

out vec4 out_color;

const float M_PI = 3.141562;
const vec3 light_dir = normalize(vec3(1.));

void main() {
    float k = max(0.0, dot(v_normal, light_dir)/M_PI);
    out_color = vec4(vec3(k), 1.0);
}
