#version 330 core

out vec4 FragColor;

in vec4 v_color;
in vec2 v_texcoord;
in vec3 v_worldpos;

uniform sampler2D u_tex;

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
}
