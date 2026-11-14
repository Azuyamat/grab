mod multi_select;

use std::{fs, io, path::Path, path::PathBuf};

use multi_select::MultiSelect;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {}

fn main() {
    let _args = Args::parse();

    match run() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

struct Entry {
    path: PathBuf,
    name: String,
    size: u64,
    modified: std::time::SystemTime,
}

impl Entry {
    fn new(path: PathBuf) -> io::Result<Self> {
        let metadata = fs::metadata(&path)?;
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?
            .to_string_lossy()
            .to_string();
        let size = metadata.len();
        let modified = metadata.modified()?;
        Ok(Self {
            path,
            name,
            size,
            modified,
        })
    }
}

impl TryFrom<PathBuf> for Entry {
    type Error = io::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Entry::new(path)
    }
}

impl AsRef<Path> for Entry {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl std::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:>8} {} - {}", self.size, self.name, self.modified
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| format!("{:?}", d))
            .unwrap_or_else(|_| "unknown".to_string()))
    }
}

fn run() -> io::Result<()> {
    let downloads_dir = get_downloads_directory()?;
    let downloads = list_downloads(&downloads_dir)?;
    if downloads.is_empty() {
        println!("No files found in the Downloads directory.");
        return Ok(());
    }

    let sorted_downloads = sort_by_modification_time(downloads);
    let multi_select = MultiSelect::new(sorted_downloads);
    let mut terminal = ratatui::init();
    let result = multi_select.run(&mut terminal);
    ratatui::restore();

    let mut error_count = 0;

    match result {
        Ok(Some(selected)) => {
            let current_dir = std::env::current_dir()?;
            let move_results = move_items(selected, &current_dir);
            for (src, result) in move_results {
                match result {
                    Ok(dest) => println!("Moved {} to {}", src.display(), dest.display()),
                    Err(e) => {
                        error_count += 1;
                        eprintln!("Failed to move {}: {}", src.display(), e);                  
                    }
                }
            }
        }
        Ok(None) => {
            println!("No items selected.");
        }
        Err(e) => {
            eprintln!("Error during selection: {}", e);
            return Err(e);
        }
    }
    
    if error_count > 0 {
        Err(io::Error::other(
            format!("{} errors occurred during file moving.", error_count),
        ))
    } else {
        Ok(())
    }
}

fn get_downloads_directory() -> io::Result<PathBuf> {
    let home = std::env::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;
    Ok(home.join("Downloads"))
}

fn list_downloads(directory: impl AsRef<Path>) -> io::Result<Vec<Entry>> {
    let entries = fs::read_dir(directory)?
        .filter_map(|entry| Entry::try_from(entry.ok()?.path()).ok())
        .collect();
    Ok(entries)
}

fn sort_by_modification_time(mut files: Vec<Entry>) -> Vec<Entry> {
    files.sort_by_key(|entry| std::cmp::Reverse(entry.modified));
    files
}

fn move_items(
    items: Vec<impl AsRef<Path>>,
    destination: impl AsRef<Path>,
) -> Vec<(PathBuf, io::Result<PathBuf>)> {
    items
        .into_iter()
        .map(|item| {
            let src = item.as_ref().to_path_buf();
            let result = move_item(&src, destination.as_ref());
            (src, result)
        })
        .collect()
}

fn move_item(src: &Path, destination_dir: &Path) -> io::Result<PathBuf> {
    let file_name = src
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;
    let dest = destination_dir.canonicalize()?.join(file_name);

    if src.canonicalize()? == dest {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Source and destination are the same",
        ));
    }
    if dest.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("Destination file {} already exists", dest.display()),
        ));
    }
    
    match fs::rename(src, &dest) {
        Ok(()) => Ok(dest),
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            if src.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Cannot move directories across different filesystems",
                ))
            }
            fs::copy(src, &dest)?;
            fs::remove_file(src)?;
            Ok(dest)
        }
        Err(e) => Err(e),
    }
}
