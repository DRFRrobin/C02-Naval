// Génère l'icône du programme (.ico) et l'intègre dans l'exécutable Windows.

#[path = "src/icon.rs"]
mod icon;

use std::io::Write;

fn ico_entry_bmp(size: u32) -> Vec<u8> {
    let rgba = icon::icon_rgba(size);
    let mut d = Vec::new();
    // BITMAPINFOHEADER (hauteur doublée : image + masque)
    d.extend_from_slice(&40u32.to_le_bytes());
    d.extend_from_slice(&(size as i32).to_le_bytes());
    d.extend_from_slice(&((size * 2) as i32).to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&32u16.to_le_bytes());
    d.extend_from_slice(&0u32.to_le_bytes());
    d.extend_from_slice(&(size * size * 4).to_le_bytes());
    d.extend_from_slice(&[0u8; 16]);
    for y in (0..size).rev() {
        for x in 0..size {
            let o = ((y * size + x) * 4) as usize;
            d.extend_from_slice(&[rgba[o + 2], rgba[o + 1], rgba[o], rgba[o + 3]]);
        }
    }
    let mask_row = size.div_ceil(32) * 4;
    d.extend(std::iter::repeat(0u8).take((mask_row * size) as usize));
    d
}

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    println!("cargo:rerun-if-changed=build.rs");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ico_path = out.join("c02-naval.ico");
    let sizes = [16u32, 24, 32, 48, 64, 128];
    let images: Vec<Vec<u8>> = sizes.iter().map(|&s| ico_entry_bmp(s)).collect();
    let mut f = std::fs::File::create(&ico_path).unwrap();
    f.write_all(&[0, 0, 1, 0]).unwrap();
    f.write_all(&(sizes.len() as u16).to_le_bytes()).unwrap();
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (s, img) in sizes.iter().zip(&images) {
        let b = if *s >= 256 { 0 } else { *s as u8 };
        f.write_all(&[b, b, 0, 0]).unwrap();
        f.write_all(&1u16.to_le_bytes()).unwrap();
        f.write_all(&32u16.to_le_bytes()).unwrap();
        f.write_all(&(img.len() as u32).to_le_bytes()).unwrap();
        f.write_all(&offset.to_le_bytes()).unwrap();
        offset += img.len() as u32;
    }
    for img in &images {
        f.write_all(img).unwrap();
    }
    drop(f);

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(ico_path.to_str().unwrap());
        res.set("ProductName", "C02 Naval");
        res.set("FileDescription", "C02 Naval - bataille navale");
        res.set("LegalCopyright", "C02 Naval");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icône non intégrée : {e}");
        }
    }
}
