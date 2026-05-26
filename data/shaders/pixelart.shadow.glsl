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
	#ifdef GLSL_ES
	out vec4 FragColor;
	#endif

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

	#ifdef GLSL_ES
		FragColor = vec4(0.0);
	#endif
}
#endif
