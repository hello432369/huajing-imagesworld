use std::path::Path;
use std::fs;
use std::io::Read;
use anyhow::Result;

pub struct Engine;

impl Engine {
    pub fn extract_image_size(path: &Path) -> Result<(u32, u32)> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("svg") {
                if let Ok(dims) = Self::extract_svg_size(path) {
                    return Ok(dims);
                }
                return Ok((0, 0));
            }
        }
        match image::ImageReader::open(path) {
            Ok(reader) => match reader.into_dimensions() {
                Ok(dims) => Ok(dims),
                Err(_) => Ok((0, 0)),
            },
            Err(_) => Ok((0, 0)),
        }
    }

    pub fn is_image_by_magic(path: &Path) -> bool {
        let mut buf = [0u8; 16];
        if fs::File::open(path).and_then(|mut f| f.read_exact(&mut buf)).is_err() {
            return false;
        }
        if buf[0] == 0xFF && buf[1] == 0xD8 && buf[2] == 0xFF { return true; }
        if buf[0] == 0x89 && buf[1] == b'P' && buf[2] == b'N' && buf[3] == b'G' { return true; }
        if buf[0] == b'G' && buf[1] == b'I' && buf[2] == b'F' && (buf[3] == b'8' || buf[3] == b'9') { return true; }
        if buf[0] == b'B' && buf[1] == b'M' { return true; }
        if buf[0] == b'R' && buf[1] == b'I' && buf[2] == b'F' && buf[3] == b'F'
            && buf[8] == b'W' && buf[9] == b'E' && buf[10] == b'B' && buf[11] == b'P' { return true; }
        if (buf[0] == 0x49 && buf[1] == 0x49 && buf[2] == 0x2A && buf[3] == 0x00)
            || (buf[0] == 0x4D && buf[1] == 0x4D && buf[2] == 0x00 && buf[3] == 0x2A) { return true; }
        if buf[0] == 0x00 && buf[1] == 0x00 && buf[2] == 0x01 && buf[3] == 0x00 { return true; }
        if buf[4] == b'f' && buf[5] == b't' && buf[6] == b'y' && buf[7] == b'p' {
            let brands: &[&[u8]] = &[b"heic", b"heim", b"heix", b"hevc", b"hevx", b"mif1", b"msf1", b"avif", b"avis"];
            if brands.contains(&&buf[8..12]) { return true; }
        }
        let buf_str = std::str::from_utf8(&buf).unwrap_or("");
        let trimmed = buf_str.trim_start();
        if trimmed.starts_with("<?xml") || trimmed.starts_with("<svg") || trimmed.starts_with("<SVG") { return true; }
        if buf[0] == 0xFE && buf[1] == 0xFF { return true; }
        if buf[0] == 0xFF && buf[1] == 0xFE { return true; }
        false
    }

    fn extract_svg_size(path: &Path) -> std::result::Result<(u32, u32), ()> {
        let content = std::fs::read_to_string(path).map_err(|_| ())?;
        if let Some(vb) = content.find("viewBox") {
            let rest = &content[vb..];
            if let Some(start) = rest.find('"') {
                let rest2 = &rest[start+1..];
                if let Some(end) = rest2.find('"') {
                    let parts: Vec<&str> = rest2[..end].split_whitespace().collect();
                    if parts.len() >= 4 {
                        if let (Ok(w), Ok(h)) = (parts[2].parse::<f32>(), parts[3].parse::<f32>()) {
                            if w > 0.0 && h > 0.0 { return Ok((w as u32, h as u32)); }
                        }
                    }
                }
            }
        }
        let try_attr = |name: &str| -> Option<u32> {
            let pos = content.find(name)?;
            let rest = &content[pos..];
            let start = rest.find('"')?;
            let end = rest[start+1..].find('"')?;
            let val = rest[start+1..start+1+end].trim();
            let num: String = val.chars().take_while(|c| c.is_digit(10) || *c == '.').collect();
            num.parse::<f32>().ok().map(|v| v as u32)
        };
        match (try_attr("width="), try_attr("height=")) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Ok((w, h)),
            _ => Err(()),
        }
    }

    pub fn detect_mime_type(path: &Path) -> String {
        match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() {
            "jpg" | "jpeg" | "jfif" | "jpe" => "image/jpeg".into(),
            "png" => "image/png".into(),
            "gif" => "image/gif".into(),
            "webp" => "image/webp".into(),
            "bmp" | "dib" => "image/bmp".into(),
            "svg" => "image/svg+xml".into(),
            "tiff" | "tif" => "image/tiff".into(),
            "heic" | "heif" | "hif" => "image/heic".into(),
            "avif" | "avifs" => "image/avif".into(),
            "ico" | "cur" => "image/x-icon".into(),
            _ => "application/octet-stream".into(),
        }
    }

    /// Generate thumbnail: max 280px, Lanczos3 resize.
    /// Returns (bytes, orig_w, orig_h, extension).
    /// PNG for images with alpha (preserves transparency), JPEG for opaque.
    pub fn generate_thumbnail(path: &Path) -> Option<(Vec<u8>, u32, u32, &'static str)> {
        let img = image::ImageReader::open(path).ok()?.decode().ok()?;
        let (w, h) = (img.width(), img.height());
        if w == 0 || h == 0 { return None; }
        let thumb_size = 280u32;
        let (tw, th) = if w > h {
            (thumb_size, (h * thumb_size / w).max(1))
        } else {
            ((w * thumb_size / h).max(1), thumb_size)
        };
        let thumb = img.resize_exact(tw, th, image::imageops::FilterType::Lanczos3);
        let mut buf = std::io::Cursor::new(Vec::new());

        if thumb.color().has_alpha() {
            // PNG with alpha — preserves transparency
            let rgba = thumb.to_rgba8();
            let dyn_img = image::DynamicImage::ImageRgba8(rgba);
            dyn_img.write_to(&mut buf, image::ImageFormat::Png).ok()?;
            Some((buf.into_inner(), w, h, "png"))
        } else {
            // JPEG — smaller for opaque images
            let rgb = thumb.to_rgb8();
            use image::codecs::jpeg::JpegEncoder;
            let mut encoder = JpegEncoder::new_with_quality(&mut buf, 92);
            encoder.encode(rgb.as_raw(), tw, th, image::ColorType::Rgb8.into()).ok()?;
            Some((buf.into_inner(), w, h, "jpg"))
        }
    }

    /// Extract dominant colors (max 9). Uses 5-bit quantization for bucketing,
    /// then computes the average actual pixel value per bucket. Skips GIFs.
    pub fn extract_colors(path: &Path, max_colors: usize) -> Vec<String> {
        if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("gif")).unwrap_or(false) {
            return vec![];
        }
        let img = match image::ImageReader::open(path).ok().and_then(|r| r.decode().ok()) {
            Some(i) => i,
            None => return vec![],
        };
        let small = img.thumbnail(64, 64);
        let rgb = small.to_rgb8();

        let mut buckets: std::collections::HashMap<u32, (u64, u64, u64, u64)> =
            std::collections::HashMap::new();
        for pixel in rgb.pixels() {
            let r = pixel[0] as u64;
            let g = pixel[1] as u64;
            let b = pixel[2] as u64;
            let key = ((r as u32 >> 3) << 10) | ((g as u32 >> 3) << 5) | (b as u32 >> 3);
            let e = buckets.entry(key).or_insert((0, 0, 0, 0));
            e.0 += r;
            e.1 += g;
            e.2 += b;
            e.3 += 1;
        }

        let mut scored: Vec<(u8, u8, u8, u64)> = buckets
            .into_iter()
            .map(|(_, (sr, sg, sb, count))| {
                let r = (sr / count) as u8;
                let g = (sg / count) as u8;
                let b = (sb / count) as u8;
                (r, g, b, count)
            })
            .collect();

        scored.sort_by(|a, b| b.3.cmp(&a.3));

        let mut result: Vec<String> = Vec::new();
        let min_distance_sq: i32 = 800;

        for &(r, g, b, _count) in &scored {
            if result.len() >= max_colors {
                break;
            }
            if r < 4 && g < 4 && b < 4 {
                continue;
            }
            if r > 251 && g > 251 && b > 251 {
                continue;
            }

            let too_close = result.iter().any(|hex| {
                let er = i32::from_str_radix(&hex[1..3], 16).unwrap_or(0);
                let eg = i32::from_str_radix(&hex[3..5], 16).unwrap_or(0);
                let eb = i32::from_str_radix(&hex[5..7], 16).unwrap_or(0);
                let dr = er - r as i32;
                let dg = eg - g as i32;
                let db = eb - b as i32;
                dr * dr + dg * dg + db * db < min_distance_sq
            });
            if too_close {
                continue;
            }

            result.push(format!("#{:02X}{:02X}{:02X}", r, g, b));
        }

        result
    }
}
