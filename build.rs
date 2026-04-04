use std::env;
use std::fs;
use std::path::PathBuf;

const DEFAULT_TSV_PATH: &str = "data/roman_lookup.tsv";
const MAGIC: &[u8; 4] = b"RLX1";

fn main() {
    println!("cargo:rerun-if-changed={DEFAULT_TSV_PATH}");

    let source = fs::read_to_string(DEFAULT_TSV_PATH).expect("default lexicon TSV must be readable");
    let compiled = compile_lexicon(&source).expect("default lexicon TSV must compile");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let output_path = out_dir.join("roman_lookup.lexicon.bin");
    fs::write(output_path, compiled).expect("compiled lexicon must be written");
}

fn compile_lexicon(source: &str) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(source.len() + 8);
    output.extend_from_slice(MAGIC);

    let entry_count = source.lines().filter(|line| !line.is_empty()).count() as u32;
    output.extend_from_slice(&entry_count.to_le_bytes());

    for (line_no, line) in source.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let Some((roman, target)) = line.split_once('\t') else {
            return Err(format!("invalid data format on line {}", line_no + 1));
        };
        if roman.contains('\0') || target.contains('\0') {
            return Err(format!("NUL byte is not supported on line {}", line_no + 1));
        }
        output.extend_from_slice(roman.as_bytes());
        output.push(0);
        output.extend_from_slice(target.as_bytes());
        output.push(0);
    }

    Ok(output)
}
