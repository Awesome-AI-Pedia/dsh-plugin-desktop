//! tar.gz / zip 解压

use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use flate2::read::GzDecoder;

pub fn extract_targz(archive: &Path, dest: &Path) -> Result<()> {
    let f = File::open(archive)
        .with_context(|| format!("打开归档失败 {}", archive.display()))?;
    let gz = GzDecoder::new(f);
    let mut ar = tar::Archive::new(gz);
    ar.set_preserve_permissions(true);
    std::fs::create_dir_all(dest).ok();
    ar.unpack(dest)
        .with_context(|| format!("解压 tar.gz 失败 → {}", dest.display()))?;
    Ok(())
}

pub fn extract_zip(archive: &Path, dest: &Path) -> Result<()> {
    let f = File::open(archive)
        .with_context(|| format!("打开归档失败 {}", archive.display()))?;
    let mut zip = zip::ZipArchive::new(f).context("读取 zip 失败")?;
    std::fs::create_dir_all(dest).ok();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let out_path = match entry.enclosed_name() {
            Some(p) => dest.join(p),
            None => continue,
        };
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).ok();
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut out = File::create(&out_path)
            .with_context(|| format!("写文件失败 {}", out_path.display()))?;
        std::io::copy(&mut entry, &mut out)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                let _ = std::fs::set_permissions(
                    &out_path,
                    std::fs::Permissions::from_mode(mode),
                );
            }
        }
    }
    Ok(())
}
