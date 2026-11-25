use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelShape {
    #[default]
    Box,
    Sphere,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelSpec {
    pub name: String,
    pub size: [f32; 3],
    pub color: [f32; 4],
    #[serde(default)]
    pub shape: ModelShape,
}
#[derive(Debug, Clone, Serialize)]
pub struct AssetReport {
    pub format: &'static str,
    pub version: String,
    pub bytes: usize,
    pub mesh_count: usize,
}

pub fn validate_gltf(bytes: &[u8]) -> Result<AssetReport, String> {
    let (format, json_bytes) = if bytes.starts_with(b"glTF") {
        if bytes.len() < 20 {
            return Err("GLB is shorter than its required header and JSON chunk".into());
        }
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let declared_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json_type = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        if version != 2 || declared_len != bytes.len() || json_type != 0x4E4F534A {
            return Err(
                "invalid GLB header: expected version 2, exact length, and JSON as first chunk"
                    .into(),
            );
        }
        if !json_len.is_multiple_of(4) || json_len > bytes.len() - 20 {
            return Err("invalid GLB JSON chunk length".into());
        }
        ("glb", &bytes[20..20 + json_len])
    } else {
        ("gltf", bytes)
    };
    let document: Value = serde_json::from_slice(json_bytes)
        .map_err(|error| format!("invalid glTF JSON: {error}"))?;
    let version = document
        .get("asset")
        .and_then(|asset| asset.get("version"))
        .and_then(Value::as_str)
        .ok_or_else(|| "glTF asset.version is required".to_owned())?;
    if version != "2.0" {
        return Err(format!("unsupported glTF version {version}; expected 2.0"));
    }
    let mesh_count = document
        .get("meshes")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    Ok(AssetReport {
        format,
        version: version.into(),
        bytes: bytes.len(),
        mesh_count,
    })
}

pub fn create_box_glb(spec: &ModelSpec) -> Result<Vec<u8>, String> {
    create_glb_mesh(spec, ModelShape::Box)
}

pub fn create_model_glb(spec: &ModelSpec) -> Result<Vec<u8>, String> {
    create_glb_mesh(spec, spec.shape)
}

fn create_glb_mesh(spec: &ModelSpec, shape: ModelShape) -> Result<Vec<u8>, String> {
    if spec.name.trim().is_empty()
        || !spec
            .size
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
        || !spec
            .color
            .into_iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
    {
        return Err(
            "model requires a non-empty name, positive finite size, and RGBA color in 0..=1".into(),
        );
    }

    let half = spec.size.map(|value| value * 0.5);
    let (positions, normals, indices) = match shape {
        ModelShape::Box => box_mesh(half),
        ModelShape::Sphere => sphere_mesh(half),
    };
    pack_glb(spec, half, &positions, &normals, &indices)
}

fn box_mesh(half: [f32; 3]) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u16>) {
    let [x, y, z] = half;
    let faces: [([[f32; 3]; 4], [f32; 3]); 6] = [
        (
            [[x, -y, -z], [x, y, -z], [x, y, z], [x, -y, z]],
            [1.0, 0.0, 0.0],
        ),
        (
            [[-x, -y, z], [-x, y, z], [-x, y, -z], [-x, -y, -z]],
            [-1.0, 0.0, 0.0],
        ),
        (
            [[-x, y, -z], [-x, y, z], [x, y, z], [x, y, -z]],
            [0.0, 1.0, 0.0],
        ),
        (
            [[-x, -y, z], [-x, -y, -z], [x, -y, -z], [x, -y, z]],
            [0.0, -1.0, 0.0],
        ),
        (
            [[x, -y, z], [x, y, z], [-x, y, z], [-x, -y, z]],
            [0.0, 0.0, 1.0],
        ),
        (
            [[-x, -y, -z], [-x, y, -z], [x, y, -z], [x, -y, -z]],
            [0.0, 0.0, -1.0],
        ),
    ];
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (corners, normal) in faces {
        positions.extend(corners);
        normals.extend([normal; 4]);
    }
    for face in 0_u16..6 {
        let base = face * 4;
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (positions, normals, indices)
}

fn sphere_mesh(half: [f32; 3]) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u16>) {
    const LATITUDE_SEGMENTS: u16 = 16;
    const LONGITUDE_SEGMENTS: u16 = 32;
    let columns = LONGITUDE_SEGMENTS + 1;
    let mut positions = Vec::with_capacity(usize::from((LATITUDE_SEGMENTS + 1) * columns));
    let mut normals = Vec::with_capacity(positions.capacity());
    let mut indices = Vec::with_capacity(usize::from(LATITUDE_SEGMENTS * LONGITUDE_SEGMENTS * 6));

    for latitude in 0..=LATITUDE_SEGMENTS {
        let phi = std::f32::consts::PI * f32::from(latitude) / f32::from(LATITUDE_SEGMENTS);
        for longitude in 0..=LONGITUDE_SEGMENTS {
            let theta =
                std::f32::consts::TAU * f32::from(longitude) / f32::from(LONGITUDE_SEGMENTS);
            let unit = [phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()];
            positions.push([unit[0] * half[0], unit[1] * half[1], unit[2] * half[2]]);
            let unnormalized = [unit[0] / half[0], unit[1] / half[1], unit[2] / half[2]];
            let length =
                (unnormalized[0].powi(2) + unnormalized[1].powi(2) + unnormalized[2].powi(2))
                    .sqrt();
            normals.push([
                unnormalized[0] / length,
                unnormalized[1] / length,
                unnormalized[2] / length,
            ]);
        }
    }

    for latitude in 0..LATITUDE_SEGMENTS {
        for longitude in 0..LONGITUDE_SEGMENTS {
            let a = latitude * columns + longitude;
            let b = a + columns;
            let c = b + 1;
            let d = a + 1;
            if latitude > 0 {
                indices.extend([a, d, b]);
            }
            if latitude + 1 < LATITUDE_SEGMENTS {
                indices.extend([b, d, c]);
            }
        }
    }
    (positions, normals, indices)
}

fn pack_glb(
    spec: &ModelSpec,
    half: [f32; 3],
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    indices: &[u16],
) -> Result<Vec<u8>, String> {
    let positions_bytes = positions.len() * 12;
    let normals_bytes = normals.len() * 12;
    let indices_bytes = indices.len() * 2;
    let mut binary = Vec::with_capacity(positions_bytes + normals_bytes + indices_bytes);
    for vertex in positions {
        for value in vertex {
            binary.extend_from_slice(&value.to_le_bytes());
        }
    }
    for normal in normals {
        for value in normal {
            binary.extend_from_slice(&value.to_le_bytes());
        }
    }
    for index in indices {
        binary.extend_from_slice(&index.to_le_bytes());
    }

    let document = json!({
        "asset": {"version": "2.0", "generator": "TRS Engine"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": spec.name, "mesh": 0}],
        "meshes": [{"name": spec.name, "primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2, "material": 0}]}],
        "materials": [{"name": format!("{}-material", spec.name), "pbrMetallicRoughness": {"baseColorFactor": spec.color, "metallicFactor": 0.0, "roughnessFactor": 0.8}}],
        "buffers": [{"byteLength": binary.len()}],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": positions_bytes, "target": 34962},
            {"buffer": 0, "byteOffset": positions_bytes, "byteLength": normals_bytes, "target": 34962},
            {"buffer": 0, "byteOffset": positions_bytes + normals_bytes, "byteLength": indices_bytes, "target": 34963}
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": positions.len(), "type": "VEC3", "min": [-half[0],-half[1],-half[2]], "max": [half[0],half[1],half[2]]},
            {"bufferView": 1, "componentType": 5126, "count": normals.len(), "type": "VEC3"},
            {"bufferView": 2, "componentType": 5123, "count": indices.len(), "type": "SCALAR"}
        ]
    });
    let mut json_bytes = serde_json::to_vec(&document).map_err(|error| error.to_string())?;
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    while binary.len() % 4 != 0 {
        binary.push(0);
    }
    let total = 12 + 8 + json_bytes.len() + 8 + binary.len();
    let mut glb = Vec::with_capacity(total);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(total as u32).to_le_bytes());
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F534A_u32.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E4942_u32.to_le_bytes());
    glb.extend_from_slice(&binary);
    Ok(glb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_glb_has_valid_container_and_mesh() {
        let glb = create_box_glb(&ModelSpec {
            name: "test-box".into(),
            size: [1.0, 2.0, 3.0],
            color: [0.2, 0.4, 0.8, 1.0],
            shape: ModelShape::Box,
        })
        .unwrap();
        let report = validate_gltf(&glb).unwrap();
        assert_eq!(report.format, "glb");
        assert_eq!(report.mesh_count, 1);
        assert_eq!(report.bytes, glb.len());
    }

    #[test]
    fn legacy_model_spec_defaults_to_box() {
        let model: ModelSpec =
            serde_json::from_str(r#"{"name":"legacy","size":[1,1,1],"color":[1,1,1,1]}"#).unwrap();
        assert_eq!(model.shape, ModelShape::Box);
    }

    #[test]
    fn sphere_model_produces_a_tessellated_glb_mesh() {
        let glb = create_model_glb(&ModelSpec {
            name: "test-sphere".into(),
            size: [1.0; 3],
            color: [1.0; 4],
            shape: ModelShape::Sphere,
        })
        .unwrap();
        let report = validate_gltf(&glb).unwrap();
        let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let document: Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
        assert_eq!(report.mesh_count, 1);
        assert_eq!(document["accessors"][0]["count"], 561);
        assert_eq!(document["accessors"][2]["count"], 2880);
        let binary_start = 20 + json_len + 8;
        let index_start =
            binary_start + document["bufferViews"][2]["byteOffset"].as_u64().unwrap() as usize;
        let first_triangle: [u16; 3] = std::array::from_fn(|index| {
            let start = index_start + index * 2;
            u16::from_le_bytes(glb[start..start + 2].try_into().unwrap())
        });
        let position = |index: u16| -> [f32; 3] {
            let start = binary_start + usize::from(index) * 12;
            std::array::from_fn(|axis| {
                let offset = start + axis * 4;
                f32::from_le_bytes(glb[offset..offset + 4].try_into().unwrap())
            })
        };
        let [a, b, c] = first_triangle.map(position);
        let edge1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let edge2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];
        let center = [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ];
        assert!(cross[0] * center[0] + cross[1] * center[1] + cross[2] * center[2] > 0.0);
    }

    #[test]
    fn asset_validation_rejects_truncated_and_wrong_version_glb() {
        assert!(validate_gltf(b"glTF").unwrap_err().contains("shorter"));
        let mut glb = create_box_glb(&ModelSpec {
            name: "x".into(),
            size: [1.0; 3],
            color: [1.0; 4],
            shape: ModelShape::Box,
        })
        .unwrap();
        glb[4..8].copy_from_slice(&1_u32.to_le_bytes());
        assert!(
            validate_gltf(&glb)
                .unwrap_err()
                .contains("invalid GLB header")
        );
    }
}
