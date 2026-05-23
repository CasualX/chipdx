#version 330 core

out vec4 FragColor;

in vec4 v_color;
in vec2 v_texcoord;
in vec3 v_worldpos;

uniform sampler2D u_tex;
uniform float u_greyscale; // 0.0 = full color, 1.0 = full greyscale

uniform sampler2D u_shadow_map;
uniform mat4 u_light_matrix;
uniform float u_shadow_bias;
uniform vec3 u_shadow_tint;

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

	color = clamp(v_color * color, 0.0, 1.0);

	float grey = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
	color.rgb = mix(color.rgb, vec3(grey), u_greyscale);

	// --- Shadow calculation ---
	vec4 light_clip = u_light_matrix * vec4(v_worldpos, 1.0);
	vec3 light_ndc  = light_clip.xyz / light_clip.w;
	vec2 light_uv = light_ndc.xy * 0.5 + 0.5;

	if (light_uv.x < 0.0 || light_uv.x > 1.0 || light_uv.y < 0.0 || light_uv.y > 1.0) {
		FragColor = color;
		return;
	}

	float current_depth = light_ndc.z * 0.5 + 0.5;
	float closest_depth = texture(u_shadow_map, light_uv).r;

	// --- PCF soft shadow ---
	vec2 texelSize = 1.0 / vec2(textureSize(u_shadow_map, 0));

	// 3x3 kernel
	float shadow = 0.0;
	for (int x = -1; x <= 1; x++) {
		for (int y = -1; y <= 1; y++) {
			vec2 offset = vec2(x, y) * texelSize;
			float depth = texture(u_shadow_map, light_uv + offset).r;
			shadow += current_depth - u_shadow_bias > depth ? 0.0 : 1.0;
		}
	}
	shadow /= 9.0;
	// shadow = mix(0.6, 1.0, shadow);

	// --- Apply shadow tint ---
	vec3 lit = color.rgb;
	vec3 shaded = color.rgb * u_shadow_tint;
	color.rgb = mix(shaded, lit, shadow);

	FragColor = color;
	// FragColor = vec4(light_uv, 0, 1);
	// FragColor = vec4(vec3(texture(u_shadow_map, light_uv).r), 1);
	// FragColor = vec4(vec3(current_depth), 1);
}
