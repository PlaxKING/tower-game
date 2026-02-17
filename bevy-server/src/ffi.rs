//! FFI C API for UE5 Integration
//!
//! Provides C-compatible functions for Protobuf deserialization.
//! UE5 calls these functions via DLL, avoiding the need for libprotobuf.lib in UE5.
use crate::proto::tower::game::ChunkData;
use prost::Message;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Deserialize Protobuf bytes to JSON string (for UE5)
///
/// # Safety
/// - `protobuf_ptr` must be a valid pointer to protobuf data
/// - `protobuf_len` must be the correct length
/// - Caller must free the returned string with `free_string()`
#[no_mangle]
pub unsafe extern "C" fn protobuf_to_json(
    protobuf_ptr: *const u8,
    protobuf_len: usize,
) -> *mut c_char {
    if protobuf_ptr.is_null() || protobuf_len == 0 {
        return ptr::null_mut();
    }

    // Convert raw pointer to slice
    let protobuf_bytes = std::slice::from_raw_parts(protobuf_ptr, protobuf_len);

    // Deserialize Protobuf
    let chunk_data = match ChunkData::decode(protobuf_bytes) {
        Ok(data) => data,
        Err(_) => return ptr::null_mut(),
    };

    // Convert to JSON using serde_json
    let json_string = match serde_json::to_string(&chunk_data) {
        Ok(json) => json,
        Err(_) => return ptr::null_mut(),
    };

    // Convert to C string
    match CString::new(json_string) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a string allocated by Rust
///
/// # Safety
/// - `ptr` must be a string allocated by `protobuf_to_json()`
/// - Must not be called twice on the same pointer
#[no_mangle]
pub unsafe extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

/// Get ChunkData field by name (for debugging)
///
/// Returns JSON representation of the field.
///
/// # Safety
/// - `protobuf_ptr` must be valid
/// - `field_name_ptr` must be valid C string
/// - Caller must free the returned string
#[no_mangle]
pub unsafe extern "C" fn get_chunk_field(
    protobuf_ptr: *const u8,
    protobuf_len: usize,
    field_name_ptr: *const c_char,
) -> *mut c_char {
    if protobuf_ptr.is_null() || field_name_ptr.is_null() {
        return ptr::null_mut();
    }

    let protobuf_bytes = std::slice::from_raw_parts(protobuf_ptr, protobuf_len);
    let field_name = match CStr::from_ptr(field_name_ptr).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let chunk_data = match ChunkData::decode(protobuf_bytes) {
        Ok(data) => data,
        Err(_) => return ptr::null_mut(),
    };

    let result = match field_name {
        "seed" => format!("{}", chunk_data.seed),
        "floor_id" => format!("{}", chunk_data.floor_id),
        "biome_id" => format!("{}", chunk_data.biome_id),
        "width" => format!("{}", chunk_data.width),
        "height" => format!("{}", chunk_data.height),
        "tiles_count" => format!("{}", chunk_data.tiles.len()),
        _ => return ptr::null_mut(),
    };

    match CString::new(result) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::tower::game::{FloorTileData, Vec3};

    #[test]
    fn test_protobuf_to_json() {
        let chunk = ChunkData {
            seed: 0x12345678,
            floor_id: 1,
            tiles: vec![FloorTileData {
                tile_type: 1,
                grid_x: 0,
                grid_y: 0,
                biome_id: 1,
                is_walkable: true,
                has_collision: false,
            }],
            validation_hash: vec![0xAB, 0xCD, 0xEF],
            biome_id: 1,
            width: 10,
            height: 10,
            world_offset: Some(Vec3 {
                x: 0.0,
                y: 5.0,
                z: 0.0,
            }),
            semantic_tags: None,
        };

        // Serialize to Protobuf
        let mut protobuf_bytes = Vec::new();
        chunk.encode(&mut protobuf_bytes).unwrap();

        // Call FFI function
        unsafe {
            let json_ptr = protobuf_to_json(protobuf_bytes.as_ptr(), protobuf_bytes.len());
            assert!(!json_ptr.is_null());

            let json_string = CStr::from_ptr(json_ptr).to_str().unwrap();
            assert!(json_string.contains("\"floor_id\":1"));
            assert!(json_string.contains("\"seed\":305419896"));

            free_string(json_ptr);
        }
    }

    #[test]
    fn test_get_chunk_field() {
        let chunk = ChunkData {
            seed: 0x12345678,
            floor_id: 42,
            tiles: vec![],
            validation_hash: vec![],
            biome_id: 1,
            width: 100,
            height: 100,
            world_offset: None,
            semantic_tags: None,
        };

        let mut protobuf_bytes = Vec::new();
        chunk.encode(&mut protobuf_bytes).unwrap();

        unsafe {
            let field_name = CString::new("floor_id").unwrap();
            let result_ptr = get_chunk_field(
                protobuf_bytes.as_ptr(),
                protobuf_bytes.len(),
                field_name.as_ptr(),
            );
            assert!(!result_ptr.is_null());

            let result_string = CStr::from_ptr(result_ptr).to_str().unwrap();
            assert_eq!(result_string, "42");

            free_string(result_ptr);
        }
    }
}
