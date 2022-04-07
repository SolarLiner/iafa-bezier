#version 330

in vec2 v_uv;
uniform sampler2D in_color;
uniform float exposure;

out vec4 out_color;

float desaturate(vec3 col) {
    return dot(col, vec3(0.2126, 0.7152, 0.0722));
}

vec3 reinhard(vec3 col) {
    return col / (1.0 + desaturate(col));
}

void main() {
    vec3 c = texture(in_color, v_uv).rgb;
    out_color = vec4(reinhard(exposure * c), 1.0);
}
