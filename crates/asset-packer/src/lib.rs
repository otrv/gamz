use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

const MAX_ASSETS: usize = 4096;
const MAX_PACK_BYTES: usize = 1024 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 8192;
const MAX_RELATIVE_PATH_BYTES: usize = 1024;

#[derive(Clone, Copy)]
pub enum OutputMode {
    Embedded,
    External,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn read_bounded(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("asset inputs must be regular files"));
    }
    let mut bytes = Vec::new();
    file.take(u64::try_from(limit).unwrap() + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid("asset input exceeds byte budget"));
    }
    Ok(bytes)
}

fn discover(sources: &Path, max_assets: usize) -> io::Result<Vec<String>> {
    let mut assets = Vec::new();
    let mut directories = vec![sources.to_path_buf()];
    let mut entries = 0;
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory)? {
            if entries == MAX_DIRECTORY_ENTRIES {
                return Err(invalid("directory entry limit exceeded"));
            }
            entries += 1;
            let entry = entry?;
            let full_path = entry.path();
            let relative = full_path.strip_prefix(sources).unwrap();
            let path = relative
                .to_str()
                .ok_or_else(|| invalid("asset paths must be UTF-8"))?
                .replace(std::path::MAIN_SEPARATOR, "/");
            if path.len() > MAX_RELATIVE_PATH_BYTES {
                return Err(invalid("asset path length limit exceeded"));
            }
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                directories.push(full_path);
                continue;
            }
            if !file_type.is_file() {
                return Err(invalid("asset inputs must be regular files, not symlinks"));
            }
            if assets.len() == max_assets {
                return Err(invalid("asset count exceeds budget"));
            }
            assets.push(path);
        }
    }
    assets.sort_unstable();
    Ok(assets)
}

fn rust_names(assets: &[String]) -> io::Result<Vec<String>> {
    let mut names = Vec::with_capacity(assets.len());
    for path in assets {
        let stem = Path::new(path).with_extension("");
        let name = stem
            .to_str()
            .unwrap()
            .replace(['/', '-'], "_")
            .to_ascii_uppercase();
        if !name.starts_with(|character: char| character.is_ascii_uppercase())
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(invalid(
                "asset path must produce an uppercase Rust identifier",
            ));
        }
        names.push(name);
    }
    let mut unique = BTreeSet::new();
    for name in &names {
        if !unique.insert(name.as_str()) {
            return Err(invalid("asset paths produce duplicate Rust identifiers"));
        }
    }
    Ok(names)
}

pub fn build(sources: &Path, output: &Path, max_assets: usize, max_bytes: usize) -> io::Result<()> {
    build_with_mode(sources, output, max_assets, max_bytes, OutputMode::Embedded)
}

pub fn build_with_mode(
    sources: &Path,
    output: &Path,
    max_assets: usize,
    max_bytes: usize,
    mode: OutputMode,
) -> io::Result<()> {
    if max_assets > MAX_ASSETS || !(16..=MAX_PACK_BYTES).contains(&max_bytes) {
        return Err(invalid(
            "pack limits must be at most 4096 assets and 16..=1073741824 bytes",
        ));
    }
    let sources = sources.canonicalize()?;
    fs::create_dir_all(output)?;
    let output = output.canonicalize()?;
    if output.starts_with(&sources) {
        return Err(invalid(
            "output directory must be outside the asset source directory",
        ));
    }
    let assets = discover(&sources, max_assets)?;
    let names = match mode {
        OutputMode::Embedded => rust_names(&assets)?,
        OutputMode::External => Vec::new(),
    };
    let catalog_end = 16 + assets.len() * 28;
    let table_length: usize = assets.iter().map(String::len).sum();
    if catalog_end + table_length > max_bytes {
        return Err(invalid("catalog exceeds pack byte budget"));
    }
    let mut pack = vec![0; catalog_end];
    pack[..4].copy_from_slice(b"GPAK");
    pack[4..8].copy_from_slice(&3_u32.to_le_bytes());
    pack[8..12].copy_from_slice(&u32::try_from(assets.len()).unwrap().to_le_bytes());
    pack[12..16].copy_from_slice(&u32::try_from(table_length).unwrap().to_le_bytes());
    for path in &assets {
        pack.extend_from_slice(path.as_bytes());
    }
    let mut name_offset = 0_u32;
    let indexed_assets = assets.iter().enumerate();
    for (index, path) in indexed_assets {
        let id = u32::try_from(index).unwrap();
        let name_length = u32::try_from(path.len()).unwrap();
        let bytes = read_bounded(&sources.join(path), max_bytes - pack.len())?;
        let offset = u64::try_from(pack.len()).unwrap();
        let length = u64::try_from(bytes.len()).unwrap();
        let record = &mut pack[16 + index * 28..16 + (index + 1) * 28];
        record[..4].copy_from_slice(&id.to_le_bytes());
        record[4..8].copy_from_slice(&name_offset.to_le_bytes());
        record[8..12].copy_from_slice(&name_length.to_le_bytes());
        record[12..20].copy_from_slice(&offset.to_le_bytes());
        record[20..28].copy_from_slice(&length.to_le_bytes());
        name_offset += name_length;
        pack.extend_from_slice(&bytes);
    }
    match mode {
        OutputMode::Embedded => write_embedded(&output.join("assets.gen.rs"), &pack, &names),
        OutputMode::External => fs::write(output.join("assets.pack"), &pack),
    }
}

fn write_embedded(path: &Path, pack: &[u8], names: &[String]) -> io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "// Auto-generated by asset-packer. Do not edit.")?;
    let records = pack[16..16 + names.len() * 28].as_chunks::<28>().0;
    let named_records = names.iter().zip(records);
    for (name, record) in named_records {
        let offset = usize::try_from(u64::from_le_bytes(record[12..20].try_into().unwrap()))
            .expect("generated pack offset fits usize");
        let length = usize::try_from(u64::from_le_bytes(record[20..28].try_into().unwrap()))
            .expect("generated pack length fits usize");
        writeln!(output, "pub(crate) static {name}: &[u8] = &[")?;
        for chunk in pack[offset..offset + length].chunks(16) {
            for byte in chunk {
                write!(output, "0x{byte:02x},")?;
            }
            writeln!(output)?;
        }
        writeln!(output, "];")?;
    }
    output.flush()
}
