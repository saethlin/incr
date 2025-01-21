use indicatif::ProgressIterator;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::ffi::OsStr;

#[derive(Debug)]
struct Experiment {
    path: PathBuf,
    offset: usize,
    time: Option<u64>,
}

impl Experiment {
    fn run<F: FnMut(&mut Experiment)>(&mut self, mut f: F) {
        cargo_build().unwrap();
        let original = fs::read_to_string(&self.path).unwrap();
        let mut new = original.clone();
        new.insert_str(self.offset, "0;");
        fs::write(&self.path, new).unwrap();
        f(self);
        fs::write(&self.path, original).unwrap();
    }
}

fn main() {
    let _initial = cargo_build().unwrap();
    let mut results = Vec::new();
    process_dir(&mut results, Path::new("src"));

    for experiment in results.iter_mut().progress() {
        experiment.run(|e| e.time = cargo_build());
    }

    results.sort_by(|a, b| a.time.cmp(&b.time));

    let last = results.last_mut().unwrap();
    last.run(|_| {
        Command::new("git").arg("diff").status().unwrap();
    });
}

fn process_dir(results: &mut Vec<Experiment>, dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let ty = entry.file_type().unwrap();
        if ty.is_file() && path.extension() == Some(OsStr::new("rs")) {
            results.extend(make_experiments(&path));
        } else if ty.is_dir() {
            process_dir(results, &path);
        }
    }
}

fn make_experiments(path: &Path) -> Vec<Experiment> {
    let re = regex::Regex::new(r#" fn .*?\{"#).unwrap();
    let mut results = Vec::new();
    let contents = fs::read_to_string(path).unwrap();
    for m in re.find_iter(&contents) {
        results.push(Experiment {
            path: PathBuf::from(path),
            offset: m.end(),
            time: None,
        });
    }
    /*
    for i in 0..contents.len() - 1 {
        if contents.get(i..i + 2) == Some("{\n") {
            results.push(Experiment {
                path: PathBuf::from(path),
                offset: i,
                time: None,
            });
        }
    }
    */
    results
}

fn cargo_build() -> Option<u64> {
    let output = Command::new("perf")
        .args(["stat", "-einstructions", "-o/tmp/time", "cargo", "build"])
        .output()
        .unwrap();
    let time = std::fs::read_to_string("/tmp/time")
        .unwrap()
        .lines()
        .nth(5)
        .unwrap()
        .split_whitespace()
        .nth(0)
        .unwrap()
        .to_string();
    let time = time.replace(",", "").parse::<u64>().unwrap();
    if output.status.success() {
        Some(time)
    } else {
        None
    }
}
