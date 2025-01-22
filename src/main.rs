use indicatif::ProgressIterator;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

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
    let mut results = Vec::new();
    process_dir(&mut results, Path::new("src"));

    let _initial = cargo_build().unwrap();
    let first = &results[0].path;
    std::fs::write(first, std::fs::read(first).unwrap()).unwrap();
    let baseline = cargo_build().unwrap();

    for experiment in results.iter_mut().progress() {
        experiment.run(|e| e.time = cargo_build());
    }

    results.sort_by(|a, b| a.time.cmp(&b.time));

    let last = results.last_mut().unwrap();
    last.run(|_| {
        Command::new("git").arg("diff").status().unwrap();
    });

    let baseline = baseline as f64;
    let mut output = BufWriter::new(File::create("results.txt").unwrap());
    for result in results.iter().filter(|e| e.time.is_some()) {
        let normalized = result.time.unwrap() as f64 / baseline;
        writeln!(output, "{}", normalized).unwrap();
    }
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
    let re = regex::Regex::new(r#"\s*fn .*?\{"#).unwrap();
    let mut results = Vec::new();
    let contents = fs::read_to_string(path).unwrap();
    for m in re.find_iter(&contents) {
        results.push(Experiment {
            path: PathBuf::from(path),
            offset: m.end(),
            time: None,
        });
    }
    results
}

fn cargo_build() -> Option<u64> {
    let file = tempfile::NamedTempFile::new().unwrap();
    let output = Command::new("perf")
        .args([
            "stat",
            "-einstructions",
            "-o",
            file.path().to_str().unwrap(),
            "cargo",
            "build",
        ])
        .output()
        .unwrap();
    let time = std::fs::read_to_string(file.path())
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
