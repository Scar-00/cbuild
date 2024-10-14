use std::fs::FileType;
use std::path::{Path, PathBuf};
use std::process::Command;
//#[cfg(feature = "tracing")]
use crate::path::Normalize;
use tracing::{event, span, Level, Span};

/*
 * TODO(S): remodel this
#[derive(Debug, Clone)]
pub enum Lang {
    C(Option<CStd>),
    Cpp(Option<CppStd>),
}

#[derive(Debug, Clone)]
pub enum CStd {
    C99,
    C11,
}

#[derive(Debug, Clone)]
pub enum CppStd {
    C99,
    C11,
}
*/

pub enum Status {
    Aborted,
    Success,
}

#[derive(Debug, Clone)]
pub enum Lang {
    C,
    Cpp,
}

#[derive(Debug, Clone)]
pub enum Std {
    C99,
    C11,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Compiler {
    Clang,
    Gcc,
    Msvc,
}

pub enum Flag {
    Optimization(OptimizationLevel),
    Debug(Option<String>),
    Files(Vec<PathBuf>),
    Definitions(Vec<String>),
    Std(String),
    Target(Target),
}

#[derive(Debug, Clone)]
pub enum Target {
    X86_64,
}

#[derive(Clone)]
pub enum OptimizationLevel {
    O3,
    O2,
    O1,
    O0,
    Small,
    Fast,
}

impl Compiler {
    const fn sys_default() -> Self {
        #[cfg(target_os = "windows")]
        return Self::Msvc;
        #[cfg(target_os = "macos")]
        return Self::Clang;
        #[cfg(target_os = "linux")]
        return Self::Gcc;
    }

    fn as_str(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Clang, Lang::C) => return "clang",
            (Self::Clang, Lang::Cpp) => return "clang++",
            (Self::Gcc, Lang::C) => return "gcc",
            (Self::Gcc, Lang::Cpp) => return "g++",
            (Self::Msvc, Lang::C | Lang::Cpp) => return "cl",
        }
    }
}

#[derive(Clone)]
pub struct CompileCommand {
    compiler: Compiler,
    lang: Lang,
    optimization_level: Option<OptimizationLevel>,
    debug: Option<String>,
    files: Vec<CompilationFile>,
    includes: Vec<PathBuf>,
    definitions: Vec<String>,
    std: Option<Std>,
    target: Option<Target>,
    out_dir: PathBuf,
    //#[cfg(feature = "tracing")]
    tracing: Span,
    dirs: Vec<PathBuf>,
    working_directory: PathBuf,
}

pub struct CompileCommandBuilder {
    inner: CompileCommand,
}

#[derive(Clone)]
pub struct CompilationFile {
    src: PathBuf,
    out: PathBuf,
}

impl Into<CompilationFile> for String {
    fn into(self) -> CompilationFile {
        CompilationFile::new(self)
    }
}

impl Into<CompilationFile> for &str {
    fn into(self) -> CompilationFile {
        CompilationFile::new(self)
    }
}

impl std::fmt::Display for CompilationFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} -> {}",
            self.src.display(),
            self.out.display()
        ))
    }
}

impl std::fmt::Debug for CompilationFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} -> {}",
            self.src.display(),
            self.out.display()
        ))
    }
}

impl CompilationFile {
    pub fn new(src: impl Into<PathBuf>) -> Self {
        let src = src.into().normalize();
        let mut out = src.clone();
        out.set_extension("o");
        return Self { src, out };
    }

    fn prepend_outdir(&mut self, out: &PathBuf) {
        //if let Some(name) = self.out.file_stem() {
        //self.out = out.join(name);
        self.out = out.join(&self.out);
        self.out.set_extension("o");
        //}
    }

    pub fn src(&self) -> &Path {
        &self.src
    }

    pub fn out(&self) -> &Path {
        &self.out
    }
}

impl CompileCommandBuilder {
    fn new() -> Self {
        return Self {
            inner: CompileCommand {
                compiler: Compiler::sys_default(),
                lang: Lang::C,
                optimization_level: None,
                debug: None,
                files: Vec::new(),
                includes: Vec::new(),
                definitions: Vec::new(),
                std: None,
                target: None,
                out_dir: "./".into(),
                //#[cfg(feature = "tracing")]
                tracing: span!(Level::INFO, "compile-command"),
                dirs: Vec::new(),
                working_directory: PathBuf::from("."),
            },
        };
    }

    fn normalize_path(&mut self) {
        let out_dir = self.inner.out_dir.join("obj");
        self.inner.files.iter_mut().for_each(|file| {
            file.prepend_outdir(&out_dir);
            if self.inner.compiler == Compiler::Msvc {
                file.out.set_extension("obj");
            }
        });
        self.inner
            .includes
            .iter_mut()
            .for_each(|file| *file = file.normalize());
    }

    pub fn build(mut self) -> CompileCommand {
        self.normalize_path();
        return self.inner;
    }

    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.inner.working_directory = dir.into();
        return self;
    }

    pub fn compiler(mut self, compiler: Compiler) -> Self {
        self.inner.compiler = compiler;
        return self;
    }

    pub fn kind(mut self, lang: Lang) -> Self {
        self.inner.lang = lang;
        return self;
    }

    pub fn opt_level(mut self, level: OptimizationLevel) -> Self {
        self.inner.optimization_level = Some(level);
        return self;
    }

    pub fn debug(mut self, debug_output: String) -> Self {
        self.inner.debug = Some(debug_output);
        return self;
    }

    pub fn out_dir(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.inner.out_dir = out_dir.into();
        return self;
    }

    pub fn file(mut self, mut file: CompilationFile) -> Self {
        if let Some(dir) = file.src.parent() {
            self.inner.dirs.push(dir.to_path_buf());
        }

        self.inner.files.push(file);
        return self;
    }

    pub fn files(mut self, files: impl Into<Vec<CompilationFile>>) -> Self {
        let mut files = files.into();
        files.iter().for_each(|file| {
            if let Some(dir) = file.src.parent() {
                self.inner.dirs.push(dir.to_path_buf());
            }
        });

        self.inner.files.extend(files);
        return self;
    }

    pub fn set_files(mut self, files: impl Into<Vec<CompilationFile>>) -> Self {
        unimplemented!();
        self.inner.files = files.into();
        return self;
    }

    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        if !dir.is_dir() {
            return self;
        }
        self.inner.dirs.push(dir.clone());
        if let Ok(dir) = dir.read_dir() {
            for file in dir {
                if let Ok(file) = file {
                    if file.path().is_dir() {
                        self = self.dir(file.path());
                    } else {
                        self.inner.files.push(CompilationFile::new(file.path()));
                    }
                }
            }
        }
        return self;
    }

    pub fn include(mut self, include: impl Into<PathBuf>) -> Self {
        self.inner.includes.push(include.into());
        return self;
    }

    pub fn includes(mut self, includes: impl Into<Vec<PathBuf>>) -> Self {
        self.inner.includes.extend(includes.into());
        return self;
    }

    pub fn set_includes(mut self, includes: impl Into<Vec<PathBuf>>) -> Self {
        self.inner.includes = includes.into();
        return self;
    }

    pub fn definition(mut self, definition: String) -> Self {
        self.inner.definitions.push(definition);
        return self;
    }

    pub fn definitions(mut self, definitions: Vec<String>) -> Self {
        self.inner.definitions.extend(definitions);
        return self;
    }

    pub fn set_definitions(mut self, definitions: Vec<String>) -> Self {
        self.inner.definitions = definitions;
        return self;
    }

    pub fn std(mut self, std: Std) -> Self {
        self.inner.std = Some(std);
        return self;
    }

    pub fn target(mut self, target: Target) -> Self {
        self.inner.target = Some(target);
        return self;
    }
}

impl CompileCommand {
    pub fn builder() -> CompileCommandBuilder {
        return CompileCommandBuilder::new();
    }

    pub fn get_link_files(&mut self) -> impl IntoIterator<Item = &PathBuf> {
        return self.files.iter().map(|file| &file.out).collect::<Vec<_>>();
    }

    pub fn gen_compiler_commands_json(&mut self) -> String {
        String::new()
    }

    pub fn files(&mut self) -> &Vec<CompilationFile> {
        &self.files
    }

    pub fn working_dir(&self) -> &Path {
        &self.working_directory
    }

    pub fn compiler(&self) -> &str {
        self.compiler.as_str(self.lang.clone())
    }

    pub fn args(&mut self) -> String {
        let mut args = String::new();
        if let Some(opt_lvl) = &self.optimization_level {
            args.push_str(", ");
            args.push_str(&format!(
                "\"{}\"",
                match opt_lvl {
                    OptimizationLevel::O3 => "-O3",
                    OptimizationLevel::O2 => "-O2",
                    OptimizationLevel::O1 => "-O1",
                    OptimizationLevel::O0 => "-O0",
                    OptimizationLevel::Fast => "-OFast",
                    OptimizationLevel::Small => "-Os",
                }
            ));
        }

        if let Some(debugger) = &self.debug {
            args.push_str(", ");
            args.push_str(&format!("\"-g{}\"", debugger));
        }
        if let Some(std) = &self.std {
            args.push_str(", ");
            args.push_str(&format!(
                "\"{}\"",
                match std {
                    Std::C99 => "-std=c99",
                    Std::C11 => "-std=c11",
                }
            ));
        }
        if let Some(_taget) = &self.target {
            args.push_str(", ");
            todo!();
        }

        for include in &self.includes {
            args.push_str(", ");
            args.push_str(&format!("\"-I\", {:?}", include));
        }

        for define in &self.definitions {
            args.push_str(", ");
            args.push_str(&format!("\"-D{}\"", define));
        }

        args
    }

    pub fn get_modified_files(&self) -> impl IntoIterator<Item = &CompilationFile> {
        self.files
            .iter()
            .filter_map(|file| {
                let (src, out) = (&file.src, &file.out);
                if !out.exists() {
                    return Some(file);
                }
                match (src.metadata(), out.metadata()) {
                    (Ok(src), Ok(out)) => match (src.modified(), out.modified()) {
                        (Ok(src), Ok(out)) => {
                            if out < src {
                                return Some(file);
                            } else {
                                return None;
                            }
                        }
                        _ => return Some(file),
                    },
                    _ => return Some(file),
                }
            })
            .collect::<Vec<_>>()
    }

    fn src_file(&self, file: &CompilationFile) -> Vec<String> {
        match self.compiler {
            Compiler::Clang | Compiler::Gcc => {
                return Vec::from([
                    "-c".into(),
                    file.src.display().to_string(),
                    "-o".into(),
                    file.out.display().to_string(),
                ]);
            }
            Compiler::Msvc => {
                return Vec::from([
                    "/c".into(),
                    file.src.display().to_string(),
                    format!("/Fo{}", file.out.display().to_string()),
                ])
            }
        }
    }

    fn opt_level(&self) -> Option<&str> {
        if let Some(opt_level) = &self.optimization_level {
            match self.compiler {
                Compiler::Clang | Compiler::Gcc => match opt_level {
                    OptimizationLevel::O3 => return Some("-O3"),
                    OptimizationLevel::O2 => return Some("-O2"),
                    OptimizationLevel::O1 => return Some("-O1"),
                    OptimizationLevel::O0 => return Some("-O0"),
                    OptimizationLevel::Fast => return Some("-OFast"),
                    OptimizationLevel::Small => return Some("-Os"),
                },
                Compiler::Msvc => match opt_level {
                    OptimizationLevel::O3 => return Some("/O2"),
                    OptimizationLevel::O2 => return Some("/O2"),
                    OptimizationLevel::O1 => return Some("/O1"),
                    OptimizationLevel::O0 => return Some("/Od"),
                    OptimizationLevel::Fast => return Some("/Ot"),
                    OptimizationLevel::Small => return Some("/Os"),
                },
            }
        }
        return None;
    }

    fn debuger(&self) -> Option<&str> {
        if let Some(debugger) = &self.debug {
            return Some(debugger.as_str());
        }
        return None;
    }

    fn lang_std(&self) -> Option<&str> {
        if let Some(std) = &self.std {
            match self.compiler {
                Compiler::Clang | Compiler::Gcc => match std {
                    Std::C99 => return Some("-std=c99"),
                    Std::C11 => return Some("-std=c11"),
                },
                Compiler::Msvc => match self.std.as_ref()? {
                    Std::C99 => {
                        let _guard = self.tracing.enter();
                        event!(Level::WARN, "failed to set language standard");
                        return None;
                    }
                    Std::C11 => return Some("/std:c11"),
                },
            }
        }
        None
    }

    fn compilation_target(&self) -> Option<&str> {
        if let Some(target) = &self.target {
            match self.compiler {
                Compiler::Clang => match target {
                    Target::X86_64 => return Some("--taget=x86-64"),
                },
                Compiler::Msvc | Compiler::Gcc => {
                    let _guard = self.tracing.enter();
                    event!(Level::WARN, "unsupported target `{:?}`", target);
                    return None;
                }
            }
        }
        None
    }

    fn include(&self, file: &Path) -> Vec<String> {
        match self.compiler {
            Compiler::Clang | Compiler::Gcc => {
                return Vec::from(["-I".into(), file.display().to_string()])
            }
            Compiler::Msvc => return Vec::from(["/I".into(), file.display().to_string()]),
        }
    }

    fn definition(&self, def: &str) -> String {
        match self.compiler {
            Compiler::Clang | Compiler::Gcc => {
                return format!("-D{}", def);
            }
            Compiler::Msvc => {
                return format!("/D{}", def);
            }
        }
    }

    fn build_command_for_file(&self, file: &CompilationFile) -> Command {
        let mut cmd = Command::new(self.compiler.as_str(self.lang.clone()));
        cmd.current_dir(&self.working_directory);
        if self.compiler == Compiler::Msvc {
            cmd.arg("/nologo");
        }
        for part in self.src_file(file) {
            cmd.arg(part);
        }
        if let Some(opt) = self.opt_level() {
            cmd.arg(opt);
        }
        if let Some(debugger) = self.debuger() {
            cmd.arg(debugger);
        }
        if let Some(std) = self.lang_std() {
            cmd.arg(std);
        }
        if let Some(target) = self.compilation_target() {
            cmd.arg(target);
        }
        for include in &self.includes {
            let include = self.include(include.as_path());
            for part in include {
                cmd.arg(part);
            }
        }
        for def in &self.definitions {
            let def = self.definition(def);
            cmd.arg(def);
        }
        cmd
    }

    fn try_create_out_dir(&self) {
        let out_dir = self.out_dir.join("obj");
        if out_dir.exists() {
            return;
        }

        let _guard = self.tracing.enter();
        event!(
            Level::INFO,
            "creating out directory: `{}`",
            out_dir.display()
        );
        if let Err(e) = std::fs::create_dir_all(&out_dir) {
            event!(Level::WARN, "failed to create out dir: `{}`", e);
        }
        for dir in &self.dirs {
            let out = out_dir.join(dir);
            std::fs::create_dir_all(out).unwrap();
        }
    }

    fn normalize_paths(&mut self) {
        let out_dir = self.out_dir.join("obj");
        self.files.iter_mut().for_each(|file| {
            file.prepend_outdir(&out_dir);
            if self.compiler == Compiler::Msvc {
                file.out.set_extension("obj");
            }
        });
        self.includes
            .iter_mut()
            .for_each(|file| *file = file.normalize());
    }

    pub fn out_dir(&mut self) -> &PathBuf {
        return &self.out_dir;
    }

    pub fn run(&mut self) -> Status {
        use std::process::Stdio;
        let _guard = self.tracing.enter();
        self.try_create_out_dir();
        for file in self.get_modified_files() {
            let mut cmd = self.build_command_for_file(file);
            println!("[Compiling]: {}", file);
            event!(Level::DEBUG, "executing: {:?}", cmd);
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
            match cmd.output() {
                Ok(out) => {
                    if !out.status.success() {
                        println!(
                            "[ERROR]: failed to compile `{}`; compilation aborted",
                            file.src.display()
                        );
                        return Status::Aborted;
                    }
                }
                Err(e) => {
                    event!(Level::WARN, "error occured: `{}`", e);
                }
            }
        }
        return Status::Success;
    }
}
