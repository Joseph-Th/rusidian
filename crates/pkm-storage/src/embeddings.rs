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

/// Compute 2D layout coordinates from high-dimensional embeddings via PCA.
///
/// Takes a collection of embedding vectors and produces 2D coordinates using
/// Principal Component Analysis (PCA). Computes the covariance matrix, finds
/// the two principal eigenvectors, and projects data onto them.
/// Results are normalized to fit [100, 900] canvas.
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

    // Center the data: compute column means
    let mut col_means = vec![0.0; d_features];
    for i in 0..n_samples {
        for j in 0..d_features {
            col_means[j] += matrix[[i, j]] as f64;
        }
    }
    for mean in &mut col_means {
        *mean /= n_samples as f64;
    }

    // Create centered matrix
    let mut centered_data = Vec::new();
    for i in 0..n_samples {
        for j in 0..d_features {
            centered_data.push((matrix[[i, j]] as f64 - col_means[j]) as f32);
        }
    }

    let centered_matrix = match Array2::from_shape_vec((n_samples, d_features), centered_data) {
        Ok(m) => m,
        Err(_) => return HashMap::new(),
    };

    // Compute covariance matrix: (X^T * X) / (n - 1)
    let centered_matrix_f64 = centered_matrix.mapv(|x| x as f64);
    let cov = centered_matrix_f64.t().dot(&centered_matrix_f64) / (n_samples as f64 - 1.0).max(1.0);

    // Compute eigenvalues and eigenvectors using power iteration for top 2
    let (eigenvalues, eigenvectors) = compute_top_eigenvalues(&cov, 2, d_features);

    if eigenvalues.len() < 2 || eigenvectors.is_empty() {
        return fallback_layout(&ids, &embedding_vecs);
    }

    // Project data onto the top 2 principal components
    let mut points_2d = Vec::new();
    for i in 0..n_samples {
        let mut x = 0.0;
        let mut y = 0.0;
        for j in 0..d_features {
            if let Some(ev_col) = eigenvectors.get(j) {
                x += centered_matrix[[i, j]] as f64 * ev_col.get(0).copied().unwrap_or(0.0);
                y += centered_matrix[[i, j]] as f64 * ev_col.get(1).copied().unwrap_or(0.0);
            }
        }
        points_2d.push((x, y));
    }

    normalize_to_canvas(&ids, points_2d)
}

/// Compute top-k eigenvalues and eigenvectors using power iteration with deflation.
/// Returns (eigenvalues, eigenvectors) where eigenvectors[j][k] is the k-th component of the j-th feature's eigenvector.
fn compute_top_eigenvalues(cov: &Array2<f64>, k: usize, d: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    let k = k.min(d);
    let mut eigenvalues = Vec::new();
    let mut eigenvectors = vec![vec![0.0; k]; d];

    // Make a mutable working copy for deflation
    let mut cov_working = cov.clone();

    // Power iteration with Hotelling deflation: find largest eigenvalue, then deflate
    for eig_idx in 0..k {
        // Start with deterministic pseudo-random vector based on seed
        let mut v: Vec<f64> = (0..d).map(|i| (i as f64 + 1.0).sin()).collect();

        // Power iteration (10 iterations usually sufficient)
        for _ in 0..10 {
            let mut av = vec![0.0; d];
            for i in 0..d {
                for j in 0..d {
                    av[i] += cov_working[[i, j]] * v[j];
                }
            }

            // Normalize
            let norm = av.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-10);
            for i in 0..d {
                v[i] = av[i] / norm;
            }
        }

        // Compute eigenvalue: v^T * A * v
        let mut lambda = 0.0;
        for i in 0..d {
            for j in 0..d {
                lambda += v[i] * cov_working[[i, j]] * v[j];
            }
        }

        lambda = lambda.max(0.0);
        eigenvalues.push(lambda);
        for j in 0..d {
            eigenvectors[j][eig_idx] = v[j];
        }

        // Hotelling deflation: A := A - λ * v * v^T
        // This removes the found eigenvector from the matrix so the next iteration finds the next one
        for i in 0..d {
            for j in 0..d {
                cov_working[[i, j]] -= lambda * v[i] * v[j];
            }
        }
    }

    (eigenvalues, eigenvectors)
}

/// Fallback layout when PCA fails: use first two dimensions directly.
fn fallback_layout(ids: &[String], embedding_vecs: &[Vec<f32>]) -> HashMap<String, (f64, f64)> {
    let mut points_2d = Vec::new();
    for emb in embedding_vecs {
        let x = emb.get(0).map(|f| *f as f64).unwrap_or(0.0);
        let y = emb.get(1).map(|f| *f as f64).unwrap_or(0.0);
        points_2d.push((x, y));
    }
    normalize_to_canvas(ids, points_2d)
}

/// Normalize 2D points to fit in [100, 900] canvas.
fn normalize_to_canvas(ids: &[String], points_2d: Vec<(f64, f64)>) -> HashMap<String, (f64, f64)> {
    // Find min/max for dynamic normalization
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for (x, y) in &points_2d {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }

    // Handle degenerate cases (all points at same location)
    let x_range = if (max_x - min_x).abs() < 1e-6 { 1.0 } else { max_x - min_x };
    let y_range = if (max_y - min_y).abs() < 1e-6 { 1.0 } else { max_y - min_y };

    // Normalize to [100, 900] canvas
    let mut result = HashMap::new();
    let canvas_min = 100.0;
    let canvas_max = 900.0;
    let canvas_size = canvas_max - canvas_min;

    for (i, id) in ids.iter().enumerate() {
        let (x, y) = points_2d[i];
        let scaled_x = ((x - min_x) / x_range) * canvas_size + canvas_min;
        let scaled_y = ((y - min_y) / y_range) * canvas_size + canvas_min;

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
