#version 330

in vec3 v_position;
in vec3 v_normal;
in vec2 v_uv;

uniform mat4 model;
uniform mat4 view_proj;
uniform mat4 inv_view_proj;

#ifdef HAS_COLOR_TEXTURE
uniform sampler2D color;
#else
uniform vec3 color;
#endif
#ifdef HAS_NORMAL_TEXTURE
uniform sampler2D normal_map;
#endif

out vec4 out_color;

const float M_PI = 3.141562;
const vec3 light_dir = normalize(vec3(1., -1., -1.));
const vec3 light_color = vec3(3.3, 3.1, 2.5);

const vec3 F0 = vec3(0.04);

vec3 fresnel(float cos_theta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

float ggx_dist(vec3 N, vec3 H, float roughness) {
    float a      = roughness*roughness;
    float a2     = a*a;
    float NdotH  = max(dot(N, H), 0.0);
    float NdotH2 = NdotH*NdotH;

    float num   = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = M_PI * denom * denom;

    return num / denom;
}

float ggx_geom(float NdotV, float roughness) {
    float r = (roughness + 1.0);
    float k = (r*r) / 8.0;

    float num   = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return num / denom;
}

float smith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2  = ggx_geom(NdotV, roughness);
    float ggx1  = ggx_geom(NdotL, roughness);

    return ggx1 * ggx2;
}

vec3 get_radiance(vec3 H, float D, vec3 N) {
    float attenuation = 1.0 / (D * D);
    return light_color * attenuation;
}

vec3 get_specular(vec3 V, vec3 H, vec3 L, float D, vec3 N) {
    float roughness = /* texture(g_rough_metal, v_uv).r */ 0.8;
    float NDF = ggx_dist(N, H, roughness);
    float G = smith(N, V, L, roughness);
    float NdotV = max(0.0, dot(N, V));
    float NdotL = max(0.0, dot(N, L));
    vec3 F = fresnel(NdotV, F0);
    vec3 num = NDF * G * F;
    float denominator = 4.0 * NdotV * NdotL + 1e-4;
    return num/denominator;
}

vec3 get_lighting(vec3 V, vec3 L, float D, vec3 N) {
    float metallic = /* texture(g_rough_metal, v_uv).g */ 0.0;

    vec3 H = normalize(V+L);
    vec3 kS = fresnel(max(0.0, dot(H, V)), F0);
    vec3 kD = (vec3(1.0) - kS) * (1.0 - metallic);
    vec3 specular = get_specular(V, H, L, D, N);
    vec3 radiance = get_radiance(H, D, N);
    float NdotL = max(0.0, dot(N, L));
    return (kD + specular) * radiance * NdotL;
}

mat3 cotangent_frame(vec3 normal, vec3 pos, vec2 uv) {
    vec3 dp1 = dFdx(pos);
    vec3 dp2 = dFdy(pos);
    vec2 duv1 = dFdx(uv);
    vec2 duv2 = dFdy(uv);
    vec3 dp2perp = cross(dp2, normal);
    vec3 dp1perp = cross(normal, dp1);
    vec3 T = dp2perp * duv1.x + dp1perp * duv2.x;
    vec3 B = dp2perp * duv1.y + dp1perp * duv2.y;
    float invmax = inversesqrt(max(dot(T, T), dot(B, B)));
    return mat3(T * invmax, B * invmax, normal);
}

float desaturate(vec3 col) {
    return dot(col, vec3(0.2126, 0.7152, 0.0722));
}

vec3 reinhard(vec3 col) {
    return col / (1.0 + desaturate(col));
}

void main() {
    vec4 view_ray4 = inv_view_proj * vec4(0.0, 0.0, -1.0, 0.0);
    vec3 view_dir = view_ray4.xyz;

    #ifdef HAS_NORMAL_TEXTURE
    mat3 tbn = cotangent_frame(v_normal, v_position, v_uv);
    vec3 tangent_map = -(texture(normal_map, v_uv).xyz * 2. - 1.);
    vec3 normal = normalize(tbn * tangent_map);
    vec3 k = get_lighting(view_dir, light_dir, 1.0, normal);
    #else
    vec3 normal = v_normal;
    vec3 k = get_lighting(view_dir, light_dir, 1.0, v_normal);
    #endif
    #ifdef HAS_COLOR_TEXTURE
    vec3 albedo = texture(color, v_uv).rgb;
    #else
    vec3 albedo = color;
    #endif
    vec3 reflectance = albedo * (k+0.1);
    out_color = vec4(reinhard(reflectance), 1.0);
    return;
}
