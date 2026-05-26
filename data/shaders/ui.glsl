#version unified 330 core, 300 es

#ifdef GLSL_ES
precision mediump float;
#endif

#ifdef VERTEX_SHADER
in vec2 a_pos;
in vec2 a_texcoord;
in vec4 a_color;
#endif

VARYING vec4 v_color;
VARYING vec2 v_texcoord;

uniform mat3x2 u_transform;
uniform vec4 u_color;
uniform float u_gamma;
uniform sampler2D u_texture;

vec3 srgbToLinear(vec3 c) {
	return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c));
}

vec4 srgbToLinear(vec4 c) {
	return vec4(srgbToLinear(c.rgb), c.a);
}

#ifdef VERTEX_SHADER
void main() {
	v_color = srgbToLinear(a_color) * u_color;
	v_texcoord = a_texcoord;
	gl_Position = vec4(u_transform * vec3(a_pos, 1.0), 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
out vec4 FragColor;

void main() {
	vec4 color = texture(u_texture, v_texcoord);
	FragColor = color * v_color;
}
#endif
