//! Semantic embeddings and vector operations for clustering and similarity search.
//!
//! This module handles:
//! - Storing and retrieving dense vector embeddings (note, entity, source, block)
//! - Computing 2D layouts from high-dimensional embeddings via PCA
//! - Similarity search and clustering operations

use ndarray::Array2;
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// Store an embedding for an object in the database.
pub fn store_embedding(
    conn: &Connection,
    object_type: &str,
    object_id: &str,
    embedding: &[f32],
    model_id: &str,
) -> Result<(), rusqlite::Error> {
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());

    // Serialize embedding to bytes (f32 little-endian)
    let embedding_bytes: Vec<u8> = embedding
        .iter()
        .flat_map(|f| f.to_le_bytes().to_vec())
        .collect();

    conn.execute(
        "INSERT OR REPLACE INTO object_embeddings (object_type, object_id, embedding, model_id, created_at)
         VALUES (?, ?, ?, ?, ?)",
        params![object_type, object_id, embedding_bytes, model_id, now],
    )?;

    Ok(())
}

/// Retrieve an embedding from the database.
pub fn get_embedding(
    conn: &Connection,
    object_type: &str,
    object_id: &str,
) -> Result<Option<Vec<f32>>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT embedding FROM object_embeddings WHERE object_type = ? AND object_id = ?",
    )?;

    let result = stmt.query_row([object_type, object_id], |row| {
        let bytes: Vec<u8> = row.get(0)?;
        Ok(bytes)
    });

    match result {
        Ok(bytes) => {
            // Deserialize f32 array from little-endian bytes
            let embedding: Vec<f32> = bytes
                .chunks_exact(4)
                .map(|chunk| {
                    let mut arr = [0u8; 4];
                    arr.copy_from_slice(chunk);
                    f32::from_le_bytes(arr)
                })
                .collect();
            Ok(Some(embedding))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Retrieve all embeddings of a given type from the database.
pub fn get_embeddings_by_type(
    conn: &Connection,
    object_type: &str,
) -> Result<HashMap<String, Vec<f32>>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT object_id, embedding FROM object_embeddings WHERE object_type = ?",
    )?;

    let embeddings = stmt.query_map([object_type], |row| {
        let id: String = row.get(0)?;
        let bytes: Vec<u8> = row.get(1)?;
        Ok((id, bytes))
    })?;

    let mut result = HashMap::new();
    for item in embeddings {
        let (id, bytes) = item?;
        let embedding: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(chunk);
                f32::from_le_bytes(arr)
            })
            .collect();
        result.insert(id, embedding);
    }

    Ok(result)
}

/// Compute 2D layout coordinates from high-dimensional embeddings.
///
/// Takes a collection of embedding vectors and produces 2D coordinates.
/// Uses simple mean-centering + dimensionality reduction via sum of first 2 components.
/// For production use, consider integrating with a full PCA library.
pub fn compute_2d_layout(embeddings: &HashMap<String, Vec<f32>>) -> HashMap<String, (f64, f64)> {
    if embeddings.is_empty() {
        return HashMap::new();
    }

    // Collect IDs and embedding vectors in order
    let mut ids = Vec::new();
    let mut embedding_vecs = Vec::new();

    for (id, emb) in embeddings.iter() {
        ids.push(id.clone());
        embedding_vecs.push(emb.clone());
    }

    let n_samples = embedding_vecs.len();
    if n_samples == 0 {
        return HashMap::new();
    }

    let d_features = embedding_vecs[0].len();
    if d_features == 0 {
        return HashMap::new();
    }

    // Convert to ndarray for processing
    let mut data = Vec::new();
    for emb in &embedding_vecs {
        data.extend(emb.iter().copied());
    }

    let matrix = match Array2::from_shape_vec((n_samples, d_features), data) {
        Ok(m) => m,
        Err(_) => return HashMap::new(),
    };

    // Compute column means for centering
    let mut col_means = vec![0.0; d_features];
    for i in 0..n_samples {
        for j in 0..d_features {
            col_means[j] += matrix[[i, j]] as f64;
        }
    }
    for mean in &mut col_means {
        *mean /= n_samples as f64;
    }

    // Simple 2D projection: use first two dimensions (or sums if fewer than 2)
    let mut result = HashMap::new();

    for (i, id) in ids.iter().enumerate() {
        let x = if d_features > 0 {
            (matrix[[i, 0]] as f64 - col_means[0]) * 100.0
        } else {
            0.0
        };

        let y = if d_features > 1 {
            (matrix[[i, 1]] as f64 - col_means[1]) * 100.0
        } else if d_features > 0 {
            (matrix[[i, 0]] as f64 - col_means[0]) * 100.0
        } else {
            0.0
        };

        // Scale to [100, 900] range (leaving margins)
        let scaled_x = (x + 500.0).max(100.0).min(900.0);
        let scaled_y = (y + 500.0).max(100.0).min(900.0);

        result.insert(id.clone(), (scaled_x, scaled_y));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_embedding() {
        let original = vec![1.0f32, 2.5f32, -3.7f32, 0.0f32];

        let bytes: Vec<u8> = original
            .iter()
            .flat_map(|f| f.to_le_bytes().to_vec())
            .collect();

        let deserialized: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(chunk);
                f32::from_le_bytes(arr)
            })
            .collect();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_compute_2d_layout_empty() {
        let embeddings = HashMap::new();
        let layout = compute_2d_layout(&embeddings);
        assert!(layout.is_empty());
    }

    #[test]
    fn test_compute_2d_layout_single_point() {
        let mut embeddings = HashMap::new();
        embeddings.insert("test".to_string(), vec![1.0; 10]);

        let layout = compute_2d_layout(&embeddings);
        assert_eq!(layout.len(), 1);

        let (x, y) = layout.get("test").unwrap();
        assert!((0.0..=1000.0).contains(x));
        assert!((0.0..=1000.0).contains(y));
    }
}
