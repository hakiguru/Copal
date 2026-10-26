//! OpenGL renderer for the 3D view. Works on OpenGL 2.1, 3.x and OpenGL ES 2/3, so Copal Lite runs
//! on old integrated graphics and on Mesa's software renderer.
//!
//! The only module with `unsafe`: every OpenGL call is unsafe in `glow`.
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;

use copal_mesh::Mesh;
use egui_glow::ShaderVersion;
use glam::{Mat3, Mat4, Vec3};
use glow::HasContext;

/// One model to draw.
#[derive(Clone)]
pub struct DrawObject {
    pub mesh: Arc<Mesh>,
    pub model: Mat4,
    /// Linear RGB, 0..1.
    pub color: [f32; 3],
    /// Color faces that need supports (models yes, supports no).
    pub overhang: bool,
}

/// A vertex with a color, for the plate, grid, volume and markers.
#[derive(Debug, Clone, Copy)]
pub struct ColorVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

/// Everything to draw in one frame.
#[derive(Clone, Default)]
pub struct Frame3d {
    pub view_proj: Mat4,
    /// Direction the light comes from (world space).
    pub light: Vec3,
    pub objects: Vec<DrawObject>,
    /// Filled triangles drawn first (plate, markers), 3 vertices each.
    pub triangles: Vec<ColorVertex>,
    /// Lines, 2 vertices each.
    pub lines: Vec<ColorVertex>,
    /// Faces looking down more steeply than this get the overhang color. `None` = off.
    pub overhang_cos: Option<f32>,
    pub overhang_color: [f32; 3],
}

struct GpuMesh {
    buffer: glow::Buffer,
    vertices: i32,
    used: bool,
    fingerprint: (usize, usize, [u32; 3]),
}

fn fingerprint(mesh: &Mesh) -> (usize, usize, [u32; 3]) {
    let first = mesh
        .positions
        .first()
        .map(|p| p.to_array().map(f32::to_bits))
        .unwrap_or_default();
    (mesh.positions.len(), mesh.triangles.len(), first)
}

pub struct Renderer {
    version: ShaderVersion,
    mesh_program: glow::Program,
    color_program: glow::Program,
    vao: Option<glow::VertexArray>,
    color_buffer: glow::Buffer,
    meshes: HashMap<usize, GpuMesh>,
    u_mvp: Option<glow::UniformLocation>,
    u_normal: Option<glow::UniformLocation>,
    u_color: Option<glow::UniformLocation>,
    u_light: Option<glow::UniformLocation>,
    u_overhang: Option<glow::UniformLocation>,
    u_overhang_color: Option<glow::UniformLocation>,
    u_color_mvp: Option<glow::UniformLocation>,
}

const MESH_VS: &str = r#"
uniform mat4 u_mvp;
uniform mat3 u_normal;
IN vec3 a_pos;
IN vec4 a_normal;
OUT vec3 v_normal;
void main() {
    v_normal = u_normal * a_normal.xyz;
    gl_Position = u_mvp * vec4(a_pos, 1.0);
}
"#;

const MESH_FS: &str = r#"
uniform vec3 u_color;
uniform vec3 u_light;
uniform float u_overhang;
uniform vec3 u_overhang_color;
IN vec3 v_normal;
void main() {
    vec3 n = normalize(v_normal);
    vec3 base = u_color;
    if (u_overhang > 0.0 && -n.z > u_overhang) {
        base = u_overhang_color;
    }
    // Back faces (inside of open or hollow models) are lit from the other side and darker.
    if (!gl_FrontFacing) {
        n = -n;
        base *= 0.55;
    }
    float key = max(dot(n, u_light), 0.0);
    float fill = max(dot(n, normalize(vec3(-u_light.x, -u_light.y, 0.6))), 0.0);
    vec3 c = base * (0.28 + 0.62 * key + 0.18 * fill);
    FRAG_COLOR = vec4(c, 1.0);
}
"#;

const COLOR_VS: &str = r#"
uniform mat4 u_mvp;
IN vec3 a_pos;
IN vec4 a_color;
OUT vec4 v_color;
void main() {
    v_color = a_color;
    gl_Position = u_mvp * vec4(a_pos, 1.0);
}
"#;

const COLOR_FS: &str = r#"
IN vec4 v_color;
void main() {
    // Premultiplied alpha, like egui.
    FRAG_COLOR = vec4(v_color.rgb * v_color.a, v_color.a);
}
"#;

fn header(version: ShaderVersion, fragment: bool) -> String {
    let mut s = version.version_declaration().to_string();
    if version.is_embedded() {
        s.push_str("precision highp float;\n");
    }
    if version.is_new_shader_interface() {
        if fragment {
            s.push_str("#define IN in\nout vec4 out_color;\n#define FRAG_COLOR out_color\n");
        } else {
            s.push_str("#define IN in\n#define OUT out\n");
        }
    } else if fragment {
        s.push_str("#define IN varying\n#define FRAG_COLOR gl_FragColor\n");
    } else {
        s.push_str("#define IN attribute\n#define OUT varying\n");
    }
    s
}

unsafe fn compile(
    gl: &glow::Context,
    version: ShaderVersion,
    vs: &str,
    fs: &str,
    attributes: &[(u32, &str)],
) -> Result<glow::Program, String> {
    unsafe {
        let program = gl.create_program()?;
        let mut shaders = Vec::new();
        for (kind, src, frag) in [
            (glow::VERTEX_SHADER, vs, false),
            (glow::FRAGMENT_SHADER, fs, true),
        ] {
            let shader = gl.create_shader(kind)?;
            gl.shader_source(shader, &format!("{}{}", header(version, frag), src));
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                return Err(format!("shader: {}", gl.get_shader_info_log(shader)));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        for (loc, name) in attributes {
            gl.bind_attrib_location(program, *loc, name);
        }
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(format!("program: {}", gl.get_program_info_log(program)));
        }
        for s in shaders {
            gl.detach_shader(program, s);
            gl.delete_shader(s);
        }
        Ok(program)
    }
}

/// Vertex data of a mesh: position (3 × f32) and face normal (4 × i8, normalized), not indexed,
/// so every face is flat-shaded.
fn mesh_vertices(mesh: &Mesh) -> Vec<u8> {
    let mut out = Vec::with_capacity(mesh.triangles.len() * 3 * 16);
    for t in &mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
        let n = (b - a).cross(c - a).normalize_or_zero();
        let packed = [
            (n.x * 127.0) as i8 as u8,
            (n.y * 127.0) as i8 as u8,
            (n.z * 127.0) as i8 as u8,
            0,
        ];
        for v in [a, b, c] {
            out.extend_from_slice(&v.x.to_ne_bytes());
            out.extend_from_slice(&v.y.to_ne_bytes());
            out.extend_from_slice(&v.z.to_ne_bytes());
            out.extend_from_slice(&packed);
        }
    }
    out
}

impl Renderer {
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        let version = ShaderVersion::get(gl);
        unsafe {
            let mesh_program = compile(
                gl,
                version,
                MESH_VS,
                MESH_FS,
                &[(0, "a_pos"), (1, "a_normal")],
            )?;
            let color_program = compile(
                gl,
                version,
                COLOR_VS,
                COLOR_FS,
                &[(0, "a_pos"), (1, "a_color")],
            )?;
            // Core profiles need a vertex array object; old OpenGL works without one.
            let vao = if version.is_new_shader_interface() {
                Some(gl.create_vertex_array()?)
            } else {
                None
            };
            Ok(Self {
                version,
                u_mvp: gl.get_uniform_location(mesh_program, "u_mvp"),
                u_normal: gl.get_uniform_location(mesh_program, "u_normal"),
                u_color: gl.get_uniform_location(mesh_program, "u_color"),
                u_light: gl.get_uniform_location(mesh_program, "u_light"),
                u_overhang: gl.get_uniform_location(mesh_program, "u_overhang"),
                u_overhang_color: gl.get_uniform_location(mesh_program, "u_overhang_color"),
                u_color_mvp: gl.get_uniform_location(color_program, "u_mvp"),
                mesh_program,
                color_program,
                vao,
                color_buffer: gl.create_buffer()?,
                meshes: HashMap::new(),
            })
        }
    }

    pub fn shader_version(&self) -> ShaderVersion {
        self.version
    }

    /// Draws a frame into the current viewport. Restores the state egui needs afterwards
    /// (egui re-applies the rest itself).
    pub fn paint(&mut self, gl: &glow::Context, frame: &Frame3d) {
        unsafe {
            if let Some(vao) = self.vao {
                gl.bind_vertex_array(Some(vao));
            }
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.depth_mask(true);
            gl.clear_depth_f32(1.0);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);

            // Plate and markers, pushed back a little so grid lines and models win.
            gl.enable(glow::POLYGON_OFFSET_FILL);
            gl.polygon_offset(1.0, 1.0);
            self.draw_colored(gl, frame, &frame.triangles, glow::TRIANGLES);
            gl.disable(glow::POLYGON_OFFSET_FILL);

            // Models.
            gl.disable(glow::BLEND);
            gl.use_program(Some(self.mesh_program));
            gl.uniform_3_f32(
                self.u_light.as_ref(),
                frame.light.x,
                frame.light.y,
                frame.light.z,
            );

            let oc = frame.overhang_color;
            gl.uniform_3_f32(self.u_overhang_color.as_ref(), oc[0], oc[1], oc[2]);
            for m in self.meshes.values_mut() {
                m.used = false;
            }
            for obj in &frame.objects {
                let key = Arc::as_ptr(&obj.mesh) as usize;
                // A freed mesh's address can be reused by a new one: check it is the same mesh.
                let print = fingerprint(&obj.mesh);
                if self
                    .meshes
                    .get(&key)
                    .is_some_and(|m| m.fingerprint != print)
                    && let Some(old) = self.meshes.remove(&key)
                {
                    gl.delete_buffer(old.buffer);
                }
                let gm = match self.meshes.entry(key) {
                    Entry::Occupied(e) => e.into_mut(),
                    Entry::Vacant(slot) => {
                        let data = mesh_vertices(&obj.mesh);
                        let Ok(buffer) = gl.create_buffer() else {
                            continue;
                        };
                        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
                        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &data, glow::STATIC_DRAW);
                        slot.insert(GpuMesh {
                            buffer,
                            vertices: (data.len() / 16) as i32,
                            used: true,
                            fingerprint: print,
                        })
                    }
                };
                gm.used = true;
                let mvp = frame.view_proj * obj.model;
                // Normals: inverse transpose of the model matrix (handles scaling and mirroring).
                let nm = Mat3::from_mat4(obj.model).inverse().transpose();
                gl.uniform_matrix_4_f32_slice(self.u_mvp.as_ref(), false, &mvp.to_cols_array());
                gl.uniform_matrix_3_f32_slice(self.u_normal.as_ref(), false, &nm.to_cols_array());
                let cos = if obj.overhang {
                    frame.overhang_cos.unwrap_or(-1.0)
                } else {
                    -1.0
                };
                gl.uniform_1_f32(self.u_overhang.as_ref(), cos);
                gl.uniform_3_f32(
                    self.u_color.as_ref(),
                    obj.color[0],
                    obj.color[1],
                    obj.color[2],
                );
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(gm.buffer));
                gl.enable_vertex_attrib_array(0);
                gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 16, 0);
                gl.enable_vertex_attrib_array(1);
                gl.vertex_attrib_pointer_f32(1, 4, glow::BYTE, true, 16, 12);
                gl.draw_arrays(glow::TRIANGLES, 0, gm.vertices);
            }
            // Free meshes that are gone (deleted objects, undone imports).
            let unused: Vec<usize> = self
                .meshes
                .iter()
                .filter(|(_, m)| !m.used)
                .map(|(k, _)| *k)
                .collect();
            for k in unused {
                if let Some(m) = self.meshes.remove(&k) {
                    gl.delete_buffer(m.buffer);
                }
            }

            gl.enable(glow::BLEND);
            gl.depth_mask(false);
            self.draw_colored(gl, frame, &frame.lines, glow::LINES);
            gl.depth_mask(true);

            gl.disable_vertex_attrib_array(0);
            gl.disable_vertex_attrib_array(1);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.disable(glow::DEPTH_TEST);
            if self.vao.is_some() {
                gl.bind_vertex_array(None);
            }
        }
    }

    unsafe fn draw_colored(
        &self,
        gl: &glow::Context,
        frame: &Frame3d,
        vertices: &[ColorVertex],
        mode: u32,
    ) {
        if vertices.is_empty() {
            return;
        }
        let mut data = Vec::with_capacity(vertices.len() * 28);
        for v in vertices {
            for f in v.pos.iter().chain(v.color.iter()) {
                data.extend_from_slice(&f.to_ne_bytes());
            }
        }
        unsafe {
            gl.use_program(Some(self.color_program));
            gl.uniform_matrix_4_f32_slice(
                self.u_color_mvp.as_ref(),
                false,
                &frame.view_proj.to_cols_array(),
            );
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.color_buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &data, glow::STREAM_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 28, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 28, 12);
            gl.draw_arrays(mode, 0, vertices.len() as i32);
        }
    }
}
