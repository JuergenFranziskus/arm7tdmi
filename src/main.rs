use std::{io::{Write, stdout}, path::PathBuf};
use arm7tdmi::tests::{Test, run_test};


const START: usize = 5;


fn main() {
    let files = get_files();          
    for file in &files[START..] {
        println!("Doing {}", file.display());
        stdout().flush().unwrap();
        let src = std::fs::read_to_string(file).unwrap();
        let tests: Vec<Test> = serde_json::de::from_str(&src).unwrap();

        for (i, test) in tests.iter().enumerate() {
            println!("Running test {i}");
            run_test(test);
            println!();
        }
    }

    println!("All done!");
}


static IGNORE: &[&str] = &[
    "arm_cdp.json",
    "arm_mcr_mrc.json",
    "arm_stc_ldc.json",
];

fn get_files() -> Vec<PathBuf> {
    let dir = PathBuf::from("./ARM7TDMI/v1");
    let mut files = Vec::new();

    for entry in dir.read_dir().unwrap() {
        let Ok(entry) = entry else { continue };
        let file = entry.path();
        let Some(ext) = file.extension() else { continue };
        let Some(ext) = ext.to_str() else { continue };
        if ext != "json" { continue; }
        let Some(name) = file.file_name().map(|n| n.to_str()).flatten() else { continue };

        let ignore = IGNORE.iter().any(|&i| i == name);
        if ignore { continue; }

        files.push(file);
    }


    files.sort();
    files
}
