#version 330

in vec3 v_normal;
in vec2 v_uv;

uniform mat4 model;
uniform mat4 view_proj;

#ifdef HAS_COLOR_TEXTURE
uniform sampler2D color;
#else
uniform vec3 color;
#endif

out vec4 out_color;

const float M_PI = 3.141562;
const vec3 light_dir = normalize(vec3(1., -1., -1.));

void main() {
    float k = max(0.0, dot(v_normal, light_dir))*0.5 + 0.3;
    #ifdef HAS_COLOR_TEXTURE
    vec3 albedo = texture(color, v_uv).rgb;
    #else
    vec3 albedo = color;
    #endif
    out_color = vec4(albedo * k, 1.0);
}
