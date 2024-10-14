pub mod compiler;
pub mod generator;
pub mod linker;
mod path;

#[cfg(test)]
mod tests {
    use super::*;
    use compiler::CompilationFile;

    #[test]
    fn test() {
        let mut command = compiler::CompileCommand::builder()
            .file(CompilationFile::new("test.c"))
            .build();

        command.run();
    }
}
