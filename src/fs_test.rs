use std::{fs::File, io::Read, path::PathBuf};

use crate::backend::{muxml2, BackendSelector};
use anyhow::Context;
use pretty_assertions::assert_str_eq;

fn read_file_to_string(file: &str) -> anyhow::Result<String> {
    let mut file_content = String::new();
    File::open(file)
        .with_context(|| format!("Failed to open {file}"))?
        .read_to_string(&mut file_content)
        .with_context(|| format!("Failed to read {file}"))?;
    Ok(file_content)
}

#[test]
#[ignore = "use insta instead"]
fn test_goldens() {
    let mut test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_dir.push("test");
    let input_dir = test_dir.join("input");
    let output_dir = test_dir.join("output");
    let muxml1_backend = BackendSelector::Muxml(());
    let muxml2_backend = BackendSelector::Muxml2(muxml2::settings::Settings {
        remove_rest_between_notes: false,
        trim_measure: false,
        simplify_time_signature: false,
    });
    for entry in std::fs::read_dir(input_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = entry.file_name();
        let filename = filename.to_str().unwrap().replace(".tab", "");
        let file_content = read_file_to_string(entry.path().to_str().unwrap()).unwrap();

        let processed_muxml1 = process(&file_content, muxml1_backend.clone()).unwrap();
        let muxml1_output_path = output_dir.join(format!("{filename}.1.musicxml"));

        let output_filename = muxml1_output_path.to_str().unwrap();
        let muxml1_expected_output = read_file_to_string(output_filename).unwrap();
        assert_str_eq!(
            muxml1_expected_output,
            processed_muxml1,
            "MUXML1 {filename} doesn't match golden {output_filename}",
        );

        // =======
        let processed_muxml2 = process(&file_content, muxml2_backend.clone()).unwrap();
        let muxml2_output_path = output_dir.join(format!("{filename}.2.musicxml"));

        let output_filename = muxml2_output_path.to_str().unwrap();
        let muxml2_expected_output = read_file_to_string(output_filename).unwrap();
        assert_str_eq!(
            muxml2_expected_output,
            processed_muxml2,
            "MUXML2 {filename} doesn't match golden {output_filename}",
        );
    }
}

fn process(s: &str, backend: BackendSelector) -> anyhow::Result<String> {
    let mut out = vec![];
    backend.process(s.lines(), &mut out);
    Ok(String::from_utf8(out)?)
}
