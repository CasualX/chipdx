#version unified 330 core, 300 es

#ifdef GLSL_ES
precision mediump float;
#endif

#ifdef VERTEX_SHADER
in vec3 a_pos;
in vec2 a_texcoord;
in vec4 a_color;
#endif

VARYING vec4 v_color;
VARYING vec2 v_texcoord;
VARYING vec3 v_worldpos;

uniform sampler2D u_tex;
uniform mat4 u_transform;
uniform float u_greyscale;
uniform sampler2D u_shadow_map;
uniform mat4 u_light_matrix;
uniform float u_shadow_bias;
uniform vec3 u_shadow_tint;
uniform vec2 u_vision_center;
uniform float u_vision_half_extent;

const float VISION_FADE_WIDTH = 16.0;
const vec3 VISION_FADE_COLOR = vec3(0.0, 6.0 / 255.0, 0.0);

vec3 srgbToLinear(vec3 c) {
	return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c));
}

vec4 srgbToLinear(vec4 c) {
	return vec4(srgbToLinear(c.rgb), c.a);
}

#ifdef VERTEX_SHADER
void main() {
	v_color = srgbToLinear(a_color);
	v_texcoord = a_texcoord / vec2(textureSize(u_tex, 0));
	v_worldpos = a_pos;
	gl_Position = u_transform * vec4(a_pos, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
out vec4 FragColor;

vec4 sample_pixelart(sampler2D tex, vec2 uv) {
	vec2 texels = uv * vec2(textureSize(tex, 0));
	vec2 sample_texels;
	#ifdef PIXELART_CRISPY
		sample_texels = floor(texels) + 0.5;
	#else
		vec2 seam = floor(texels + 0.5);
		vec2 footprint = max(fwidth(texels), vec2(1e-6));
		sample_texels = seam + clamp((texels - seam) / footprint, -0.5, 0.5);
	#endif
	return texture(tex, sample_texels / vec2(textureSize(tex, 0)));
}

void main() {
	vec4 color = sample_pixelart(u_tex, v_texcoord);
	if (color.a < 0.2) {
		discard;
	}
	vec2 delta = abs(v_worldpos.xy - u_vision_center);
	if (delta.x > u_vision_half_extent || delta.y > u_vision_half_extent) {
		discard;
	}
	vec2 edge_dist = u_vision_half_extent - delta;
	vec2 edge_t = clamp(edge_dist / VISION_FADE_WIDTH, 0.0, 1.0);
	vec2 edge_fade_axis = 1.0 - pow(1.0 - edge_t, vec2(3.0));
	float edge_fade = edge_fade_axis.x * edge_fade_axis.y;

	color = clamp(v_color * color, 0.0, 1.0);

	float grey = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
	color.rgb = mix(color.rgb, vec3(grey), u_greyscale);

	vec4 light_clip = u_light_matrix * vec4(v_worldpos, 1.0);
	vec3 light_ndc = light_clip.xyz / light_clip.w;
	vec2 light_uv = light_ndc.xy * 0.5 + 0.5;

	if (light_uv.x < 0.0 || light_uv.x > 1.0 || light_uv.y < 0.0 || light_uv.y > 1.0) {
		color.rgb = mix(VISION_FADE_COLOR, color.rgb, edge_fade);
		FragColor = color;
		return;
	}

	float current_depth = light_ndc.z * 0.5 + 0.5;
	vec2 texelSize = 1.0 / vec2(textureSize(u_shadow_map, 0));

	float shadow = 0.0;
	for (int x = -1; x <= 1; x++) {
		for (int y = -1; y <= 1; y++) {
			vec2 offset = vec2(float(x), float(y)) * texelSize;
			float depth = texture(u_shadow_map, light_uv + offset).r;
			shadow += current_depth - u_shadow_bias > depth ? 0.0 : 1.0;
		}
	}
	shadow /= 9.0;

	vec3 lit = color.rgb;
	vec3 shaded = color.rgb * u_shadow_tint;
	color.rgb = mix(shaded, lit, shadow);
	color.rgb = mix(VISION_FADE_COLOR, color.rgb, edge_fade);

	FragColor = color;
}
#endif
