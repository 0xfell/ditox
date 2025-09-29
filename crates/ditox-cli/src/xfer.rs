use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::{Query, Store};
use image::ImageEncoder;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum ClipExport {
    Text {
        id: String,
        created_at: i64,
        favorite: bool,
        text: String,
        tags: Vec<String>,
    },
    Image {
        id: String,
        created_at: i64,
        favorite: bool,
        tags: Vec<String>,
        image: ImageExport,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImageExport {
    sha256: String,
    format: String,
    width: u32,
    height: u32,
    size_bytes: u64,
}

pub fn export_all(
    store: &dyn Store,
    dir: &Path,
    favorites: bool,
    images_only: bool,
    tag: Option<&str>,
) -> Result<()> {
    fs::create_dir_all(dir)?;
    let mut out = fs::File::create(dir.join("clips.jsonl"))?;
    let tag = tag.map(|s| s.to_string());
    if images_only {
        for (c, m) in store.list_images(Query {
            contains: None,
            favorites_only: favorites,
            limit: None,
            tag: tag.clone(),
            rank: false,
        })? {
            let cid = c.id.clone();
            let exp = ClipExport::Image {
                id: c.id,
                created_at: c.created_at.unix_timestamp(),
                favorite: c.is_favorite,
                tags: store.list_tags(&cid).unwrap_or_default(),
                image: ImageExport {
                    sha256: m.sha256,
                    format: m.format,
                    width: m.width,
                    height: m.height,
                    size_bytes: m.size_bytes,
                },
            };
            // write image blob
            if let Some(img) = store.get_image_rgba(&cid)? {
                let mut buf = Vec::new();
                image::codecs::png::PngEncoder::new(&mut buf).write_image(
                    &img.bytes,
                    img.width,
                    img.height,
                    image::ExtendedColorType::Rgba8,
                )?;
                let (a, b) = (&exp_sha(&exp)[0..2], &exp_sha(&exp)[2..4]);
                let obj = dir.join("objects").join(a).join(b);
                fs::create_dir_all(&obj)?;
                let path = obj.join(exp_sha(&exp));
                if !path.exists() {
                    fs::write(path, &buf)?;
                }
            }
            writeln!(out, "{}", serde_json::to_string(&exp)?)?;
        }
    } else {
        // text
        for c in store.list(Query {
            contains: None,
            favorites_only: favorites,
            limit: None,
            tag: tag.clone(),
            rank: false,
        })? {
            let exp = ClipExport::Text {
                id: c.id.clone(),
                created_at: c.created_at.unix_timestamp(),
                favorite: c.is_favorite,
                text: c.text,
                tags: store.list_tags(&c.id).unwrap_or_default(),
            };
            writeln!(out, "{}", serde_json::to_string(&exp)?)?;
        }
        // images as well
        for (c, m) in store.list_images(Query {
            contains: None,
            favorites_only: favorites,
            limit: None,
            tag,
            rank: false,
        })? {
            let exp = ClipExport::Image {
                id: c.id.clone(),
                created_at: c.created_at.unix_timestamp(),
                favorite: c.is_favorite,
                tags: store.list_tags(&c.id).unwrap_or_default(),
                image: ImageExport {
                    sha256: m.sha256,
                    format: m.format,
                    width: m.width,
                    height: m.height,
                    size_bytes: m.size_bytes,
                },
            };
            if let Some(img) = store.get_image_rgba(&c.id)? {
                let mut buf = Vec::new();
                image::codecs::png::PngEncoder::new(&mut buf).write_image(
                    &img.bytes,
                    img.width,
                    img.height,
                    image::ExtendedColorType::Rgba8,
                )?;
                let (a, b) = (&exp_sha(&exp)[0..2], &exp_sha(&exp)[2..4]);
                let obj = dir.join("objects").join(a).join(b);
                fs::create_dir_all(&obj)?;
                let path = obj.join(exp_sha(&exp));
                if !path.exists() {
                    fs::write(path, &buf)?;
                }
            }
            writeln!(out, "{}", serde_json::to_string(&exp)?)?;
        }
    }
    Ok(())
}

pub fn import_all(store: &dyn Store, path: &Path, keep_ids: bool) -> Result<usize> {
    let mut imported = 0usize;
    if path.is_dir() {
        let f = fs::File::open(path.join("clips.jsonl"))?;
        let mut rdr = BufReader::new(f);
        let mut line = String::new();
        while rdr.read_line(&mut line)? > 0 {
            if line.trim().is_empty() {
                line.clear();
                continue;
            }
            imported += import_one(store, path, &line, keep_ids)?;
            line.clear();
        }
    } else {
        let s = fs::read_to_string(path)?;
        for l in s.lines() {
            if l.trim().is_empty() {
                continue;
            }
            let base = path.parent().unwrap_or(Path::new("."));
            imported += import_one(store, base, l, keep_ids)?;
        }
    }
    Ok(imported)
}

fn import_one(store: &dyn Store, base: &Path, line: &str, keep_ids: bool) -> Result<usize> {
    let v: ClipExport = serde_json::from_str(line)?;
    match v {
        ClipExport::Text { id, text, .. } => {
            let c = store.add(&text)?;
            if keep_ids && c.id != id { /* ignore id mapping for now */ }
            Ok(1)
        }
        ClipExport::Image { image, .. } => {
            let (a, b) = (&image.sha256[0..2], &image.sha256[2..4]);
            let path = base.join("objects").join(a).join(b).join(&image.sha256);
            let bytes = fs::read(&path)?;
            let img = image::load_from_memory(&bytes)?;
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let _ = store.add_image_rgba(w, h, &rgba.into_raw())?;
            Ok(1)
        }
    }
}

fn exp_sha(exp: &ClipExport) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(exp).unwrap());
    hex::encode(hasher.finalize())
}
