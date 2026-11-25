use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelSpec {
    pub name: String,
    pub size: [f32; 3],
    pub color: [f32; 4],
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

    let [x, y, z] = spec.size.map(|value| value * 0.5);
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
    let mut binary = Vec::with_capacity(648);
    for (corners, normal) in faces {
        for position in corners {
            for value in position {
                binary.extend_from_slice(&value.to_le_bytes());
            }
            for value in normal {
                binary.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    for face in 0_u16..6 {
        let base = face * 4;
        for index in [base, base + 1, base + 2, base, base + 2, base + 3] {
            binary.extend_from_slice(&index.to_le_bytes());
        }
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
            {"buffer": 0, "byteOffset": 0, "byteLength": 288, "target": 34962},
            {"buffer": 0, "byteOffset": 288, "byteLength": 288, "target": 34962},
            {"buffer": 0, "byteOffset": 576, "byteLength": 72, "target": 34963}
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 24, "type": "VEC3", "min": [-x,-y,-z], "max": [x,y,z]},
            {"bufferView": 1, "componentType": 5126, "count": 24, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5123, "count": 36, "type": "SCALAR"}
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
        })
        .unwrap();
        let report = validate_gltf(&glb).unwrap();
        assert_eq!(report.format, "glb");
        assert_eq!(report.mesh_count, 1);
        assert_eq!(report.bytes, glb.len());
    }

    #[test]
    fn asset_validation_rejects_truncated_and_wrong_version_glb() {
        assert!(validate_gltf(b"glTF").unwrap_err().contains("shorter"));
        let mut glb = create_box_glb(&ModelSpec {
            name: "x".into(),
            size: [1.0; 3],
            color: [1.0; 4],
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
