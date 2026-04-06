// This is the Glyph vertex shader.
// It is responsible for placing the glyph images in the
// correct place on screen.

precision highp float;

in vec2 position;
in vec2 tex;
in vec4 fg_color;
in float has_color;
in float mix_value;
in vec3 hsv;
in vec4 alt_color;
in vec4 clip_rect;
in float clip_radius;
in float clip_enabled;

uniform mat4 projection;

out float o_has_color;
out vec2 o_tex;
out vec3 o_hsv;
out vec4 o_fg_color;
out vec4 o_fg_color_alt;
out float o_fg_color_mix;
out vec2 o_position;
out vec4 o_clip_rect;
out float o_clip_radius;
out float o_clip_enabled;

void pass_through_vertex() {
  o_tex = tex;
  o_has_color = has_color;
  o_fg_color = fg_color;
  o_fg_color_alt = alt_color;
  o_fg_color_mix = mix_value;
  o_hsv = hsv;
  o_position = position;
  o_clip_rect = clip_rect;
  o_clip_radius = clip_radius;
  o_clip_enabled = clip_enabled;
}

void main() {
  pass_through_vertex();

  // Use the adjusted cell position to render the quad
  gl_Position = projection * vec4(position, 0.0, 1.0);
}
