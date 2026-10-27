use game_core::render::{self, RenderError};
use math::{Mat4, Radians, Vec3, Vec4};
use platform_api::render::{RenderCommand, RenderCommands, Vertex3D};

pub(super) enum DrawOrder {
    Forward,
    Reverse,
}

struct Mesh {
    vertices: &'static [Vertex3D],
    indices: &'static [u32],
    position: Vec3,
}

const CUBE_INDICES: [u32; 36] = [
    0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17, 18,
    16, 18, 19, 20, 21, 22, 20, 22, 23,
];
const PLAYER_VERTICES: [Vertex3D; 24] =
    box_vertices(Vec3::new(1.0, 2.0, 1.0), Vec4::new(0.95, 0.95, 1.0, 1.0));
const FLOOR_COLOR: Vec4 = Vec4::new(0.22, 0.29, 0.34, 1.0);
const FLOOR: [Vertex3D; 4] = [
    Vertex3D {
        position: Vec3::new(-7.0, 0.0, -4.0),
        color: FLOOR_COLOR,
    },
    Vertex3D {
        position: Vec3::new(-7.0, 0.0, 11.0),
        color: FLOOR_COLOR,
    },
    Vertex3D {
        position: Vec3::new(7.0, 0.0, 11.0),
        color: FLOOR_COLOR,
    },
    Vertex3D {
        position: Vec3::new(7.0, 0.0, -4.0),
        color: FLOOR_COLOR,
    },
];
const MESHES: [Mesh; 5] = [
    Mesh {
        vertices: &FLOOR,
        indices: &[0, 1, 2, 0, 2, 3],
        position: Vec3::new(0.0, 0.0, 0.0),
    },
    Mesh {
        vertices: &box_vertices(Vec3::new(2.0, 2.0, 2.0), Vec4::new(1.0, 0.4, 0.22, 1.0)),
        indices: &CUBE_INDICES,
        position: Vec3::new(-2.0, 0.0, 1.0),
    },
    Mesh {
        vertices: &box_vertices(Vec3::new(2.0, 2.0, 2.0), Vec4::new(0.25, 0.75, 1.0, 1.0)),
        indices: &CUBE_INDICES,
        position: Vec3::new(0.0, 0.0, 3.0),
    },
    Mesh {
        vertices: &box_vertices(Vec3::new(2.0, 2.0, 2.0), Vec4::new(1.0, 0.8, 0.25, 1.0)),
        indices: &CUBE_INDICES,
        position: Vec3::new(2.5, 0.0, 1.5),
    },
    Mesh {
        vertices: &box_vertices(Vec3::new(2.0, 2.0, 2.0), Vec4::new(0.4, 0.9, 0.55, 1.0)),
        indices: &CUBE_INDICES,
        position: Vec3::new(0.0, 0.0, 6.0),
    },
];

pub(super) fn draw(
    order: &DrawOrder,
    player_position: Vec3,
    commands: &mut RenderCommands<'_>,
) -> Result<(), RenderError> {
    let player = Mesh {
        vertices: &PLAYER_VERTICES,
        indices: &CUBE_INDICES,
        position: player_position,
    };
    let view = Mat4::look_at_lh(
        Vec3::new(player_position.x, 7.0, player_position.z - 10.0),
        Vec3::new(player_position.x, 1.0, player_position.z),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let projection = Mat4::perspective_lh(
        Radians::new(core::f32::consts::FRAC_PI_3),
        1280.0 / 720.0,
        0.1,
        100.0,
    );
    render::push(
        commands,
        RenderCommand::SetCamera3D {
            view_projection: projection * view,
        },
    )?;
    for index in 0..=MESHES.len() {
        let index = match order {
            DrawOrder::Forward => index,
            DrawOrder::Reverse => MESHES.len() - index,
        };
        let mesh = if index == MESHES.len() {
            &player
        } else {
            &MESHES[index]
        };
        render::push(
            commands,
            RenderCommand::DrawMesh {
                vertices: mesh.vertices,
                indices: mesh.indices,
                model: Mat4::from_translation(mesh.position),
            },
        )?;
    }
    Ok(())
}

const fn box_vertices(size: Vec3, color: Vec4) -> [Vertex3D; 24] {
    let half_width = size.x * 0.5;
    let half_depth = size.z * 0.5;
    let corners = [
        Vec3::new(-half_width, 0.0, -half_depth),
        Vec3::new(half_width, 0.0, -half_depth),
        Vec3::new(half_width, size.y, -half_depth),
        Vec3::new(-half_width, size.y, -half_depth),
        Vec3::new(-half_width, 0.0, half_depth),
        Vec3::new(half_width, 0.0, half_depth),
        Vec3::new(half_width, size.y, half_depth),
        Vec3::new(-half_width, size.y, half_depth),
    ];
    let faces = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 4, 7, 3],
        [1, 2, 6, 5],
        [3, 7, 6, 2],
        [0, 1, 5, 4],
    ];
    let shades = [0.75, 0.75, 0.6, 0.6, 1.0, 0.5];
    let mut vertices = [Vertex3D {
        position: Vec3::new(0.0, 0.0, 0.0),
        color,
    }; 24];
    let mut face = 0;
    while face < faces.len() {
        let shade = shades[face];
        let mut corner = 0;
        while corner < 4 {
            vertices[face * 4 + corner] = Vertex3D {
                position: corners[faces[face][corner]],
                color: Vec4::new(color.x * shade, color.y * shade, color.z * shade, color.w),
            };
            corner += 1;
        }
        face += 1;
    }
    vertices
}
