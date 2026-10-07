use std::path::{Path, PathBuf};

pub fn create_new_file_with_rotation(
    file_path: &PathBuf,
) -> std::io::Result<std::fs::File> {
    let parent_dir = file_path.parent().unwrap();
    std::fs::create_dir_all(parent_dir)?;

    if file_path.exists() {
        let mut i = 1;
        while std::fs::exists(Path::new(format!(
            "{}.{}",
            file_path.to_str().unwrap(),
            i
        ).as_str()))? {
            i += 1;
        }
        let to_filename = format!(
            "{}.{}",
            file_path.to_str().unwrap(),
            i
        );
        let to = Path::new(to_filename.as_str());
        std::fs::rename(file_path, to)?;
    }

    std::fs::File::create(file_path)
}