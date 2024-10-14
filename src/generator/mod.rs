use crate::compiler::{CompilationFile, CompileCommand};
use std::path::PathBuf;

pub struct ConfigGenerator<'a> {
    compiler_command: &'a mut CompileCommand,
    root_dir: PathBuf,
}

impl<'a> ConfigGenerator<'a> {
    pub fn new(compiler_command: &'a mut CompileCommand, root_dir: PathBuf) -> Self {
        return Self {
            compiler_command,
            root_dir,
        };
    }

    fn generate_file(&self, file: &CompilationFile, command: &mut CompileCommand) -> String {
        let mut entry = String::new();
        entry.push_str("{");
        entry.push_str(&format!("\"directory\": {:?},", self.root_dir));
        entry.push_str(&format!("\"file\": {:?},", file.src()));
        entry.push_str(&format!("\"output\": {:?},", file.out()));
        entry.push_str(&format!("\"arguments\": [\"{}\", ", command.compiler()));
        entry.push_str(&format!(
            "\"-c\", {:?}, \"-o\", {:?}",
            file.src(),
            file.out()
        ));
        entry.push_str(&command.args());
        entry.push_str("]");

        entry.push_str("}");
        entry
    }

    pub fn generate(&mut self) -> String {
        let mut content = String::new();
        content.push_str("[\n");
        let mut command = self.compiler_command.clone();
        for (i, file) in self.compiler_command.files().clone().iter().enumerate() {
            if i != 0 {
                content.push_str(", ");
            }
            let file = self.generate_file(file, &mut command);
            content.push_str(&file);
        }

        content.push_str("\n]");
        content
    }
}
