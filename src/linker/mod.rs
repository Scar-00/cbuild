use crate::compiler::*;
use crate::path::Normalize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::{event, span, Level};

#[derive(PartialEq)]
pub enum BinType {
    StaticLib,
    DynamicLib,
    Binary,
}

#[derive(PartialEq)]
pub enum Linker {
    Clang,
    #[allow(non_camel_case_types)]
    LLVM_LD,
    Gcc,
    Ld,
    Link,
}

impl Linker {
    const fn sys_default() -> Self {
        #[cfg(target_os = "windows")]
        return Self::Link;
        #[cfg(target_os = "macos")]
        return Self::LLVM_LD;
        #[cfg(target_os = "linux")]
        return Self::Ld;
    }

    /*fn as_str(&self) -> &str {
        match self {
            Linker::Clang => return "clang",
            Linker::LLVM_LD => return "lld",
            Linker::Gcc => return "gcc",
            Linker::Ld => return "ld",
            Linker::Link => return "link",
        }
    }*/
}

pub struct LinkerCommandBuilder<'a> {
    inner: LinkerCommand<'a>,
}

const fn default_exec_name() -> &'static str {
    return "a.exe";
}

impl<'a> LinkerCommandBuilder<'a> {
    fn new(command: &'a mut CompileCommand) -> Self {
        return Self {
            inner: LinkerCommand {
                compile_command: command,
                bin_type: BinType::Binary,
                linker: Linker::sys_default(),
                name: default_exec_name().to_string(),
                links: Vec::new(),
                link_dirs: Vec::new(),
                flags: Vec::new(),
                link_sys_deafult: false,
            },
        };
    }

    pub fn build(self) -> LinkerCommand<'a> {
        self.inner
    }

    pub fn bin_type(mut self, bin_type: BinType) -> Self {
        self.inner.bin_type = bin_type;
        self
    }

    pub fn link_sys_deafult(mut self, link: bool) -> Self {
        self.inner.link_sys_deafult = link;
        return self;
    }

    pub fn name(mut self, name: impl ToString) -> Self {
        self.inner.name = name.to_string();
        return self;
    }

    pub fn linker(mut self, linker: Linker) -> Self {
        self.inner.linker = linker;
        return self;
    }

    pub fn link(mut self, link: impl Into<PathBuf>) -> Self {
        let link = link.into().normalize();
        self.inner.links.push(link);
        return self;
    }

    pub fn links(mut self, links: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        let links = links
            .into_iter()
            .map(|link| link.into().normalize())
            .collect::<Vec<_>>();
        self.inner.links.extend(links);
        return self;
    }

    pub fn set_links(mut self, links: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        let links = links
            .into_iter()
            .map(|link| link.into().normalize())
            .collect::<Vec<_>>();
        self.inner.links = links;
        return self;
    }

    pub fn link_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.inner.link_dirs.push(dir.into());
        return self;
    }

    pub fn link_dirs(mut self, dirs: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        let dirs = dirs.into_iter().map(|dir| dir.into()).collect::<Vec<_>>();
        self.inner.link_dirs.extend(dirs);
        return self;
    }

    pub fn set_link_dirs(mut self, dirs: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        let dirs = dirs.into_iter().map(|dir| dir.into()).collect::<Vec<_>>();
        self.inner.link_dirs = dirs;
        return self;
    }

    pub fn flag(mut self, flag: impl ToString) -> Self {
        self.inner.flags.push(flag.to_string());
        return self;
    }

    pub fn flags(mut self, flags: impl IntoIterator<Item = impl ToString>) -> Self {
        let flags = flags
            .into_iter()
            .map(|flag| flag.to_string())
            .collect::<Vec<_>>();
        self.inner.flags.extend(flags);
        return self;
    }

    pub fn set_flags(mut self, flags: impl IntoIterator<Item = impl ToString>) -> Self {
        let flags = flags
            .into_iter()
            .map(|flag| flag.to_string())
            .collect::<Vec<_>>();
        self.inner.flags = flags;
        return self;
    }
}

pub struct LinkerCommand<'a> {
    compile_command: &'a mut CompileCommand,
    bin_type: BinType,
    linker: Linker,
    name: String,
    links: Vec<PathBuf>,
    link_dirs: Vec<PathBuf>,
    flags: Vec<String>,
    link_sys_deafult: bool,
}

impl<'a> LinkerCommand<'a> {
    pub fn builder(command: &'a mut CompileCommand) -> LinkerCommandBuilder {
        return LinkerCommandBuilder::new(command);
    }

    const fn get_sys_deafault_libs() -> &'static [&'static str] {
        return &[];
    }

    fn to_out(&mut self) -> Vec<String> {
        let mut name = PathBuf::from(&self.name);
        name.set_extension(self.get_exec_ext());
        let out = self.compile_command.out_dir().join(name);
        match self.linker {
            Linker::Clang | Linker::Gcc | Linker::Ld | Linker::LLVM_LD => {
                return Vec::from(["-o".into(), out.display().to_string()])
            }
            Linker::Link => return Vec::from([format!("/OUT:{}", out.display().to_string())]),
        }
    }

    pub fn out_file(&mut self) -> PathBuf {
        self.compile_command.out_dir().join(&self.name)
    }

    fn link_file(&self, file: &PathBuf) -> Vec<String> {
        match self.linker {
            Linker::Clang | Linker::Ld | Linker::Gcc | Linker::LLVM_LD => {
                return Vec::from(["-l".into(), file.display().to_string()])
            }
            Linker::Link => return Vec::from([file.display().to_string()]),
        }
    }

    fn link_dir(&self, dir: &PathBuf) -> Vec<String> {
        match self.linker {
            Linker::Clang | Linker::Ld | Linker::Gcc | Linker::LLVM_LD => {
                return Vec::from(["-L".into(), dir.display().to_string()])
            }
            Linker::Link => return Vec::from([format!("/LIBPATH:{}", dir.display().to_string())]),
        }
    }

    const fn get_exec_ext(&self) -> &'static str {
        match self.bin_type {
            #[cfg(target_os = "windows")]
            BinType::Binary => "exe",
            #[cfg(target_os = "macos")]
            BinType::Binary => "",
            #[cfg(target_os = "linux")]
            BinType::Binary => "",
            #[cfg(target_os = "windows")]
            BinType::StaticLib => "lib",
            #[cfg(target_os = "macos")]
            BinType::StaticLib => "a",
            #[cfg(target_os = "linux")]
            BinType::StaticLib => "a",
            #[cfg(target_os = "windows")]
            BinType::DynamicLib => "dll",
            #[cfg(target_os = "macos")]
            BinType::DynamicLib => "so",
            #[cfg(target_os = "linux")]
            BinType::DynamicLib => "so",
        }
    }

    fn should_rerun(&mut self) -> bool {
        let time = if let Ok(metadata) = self.out_file().metadata() {
            if let Ok(time) = metadata.modified() {
                time
            } else {
                return true;
            }
        } else {
            return true;
        };
        for file in self.compile_command.get_link_files() {
            if let Ok(metadata) = file.metadata() {
                if let Ok(out_time) = metadata.modified() {
                    if time < out_time {
                        return true;
                    }
                } else {
                    return true;
                }
            } else {
                return true;
            }
        }
        false
    }

    /*
       Linker::Clang => return "clang",
       Linker::LLVM_LD => return "lld",
       Linker::Gcc => return "gcc",
       Linker::Ld => return "ld",
       Linker::Link => return "link",
    */

    fn linker(&self) -> &'static str {
        match (&self.linker, &self.bin_type) {
            (Linker::Link, BinType::Binary) | (Linker::Link, BinType::DynamicLib) => "link.exe",
            (Linker::Link, BinType::StaticLib) => "lib.exe",
            (Linker::Clang, BinType::Binary) | (Linker::Clang, BinType::DynamicLib) => "clang",
            (Linker::Clang, BinType::StaticLib) => "llvm-ar",
            (Linker::LLVM_LD, _) => "lld",
            (Linker::Ld, _) => "ld",
            _ => todo!(),
        }
    }

    fn shared_flag(&self) -> &'static str {
        match &self.linker {
            Linker::Link => "/DLL",
            Linker::Clang => "-shared",
            _ => todo!(),
        }
    }

    fn build_command(&mut self) -> Command {
        let mut cmd = Command::new(self.linker());
        cmd.current_dir(self.compile_command.working_dir());
        if self.bin_type == BinType::DynamicLib {
            cmd.arg(self.shared_flag());
        }

        if self.linker == Linker::Link {
            cmd.arg("/nologo");
        }
        for part in self.to_out() {
            cmd.arg(part);
        }
        for dir in &self.link_dirs {
            for part in self.link_dir(dir) {
                cmd.arg(part);
            }
        }
        for file in self.compile_command.get_link_files() {
            cmd.arg(file);
        }
        for link in &self.links {
            for part in self.link_file(link) {
                cmd.arg(part);
            }
        }
        for flag in &self.flags {
            cmd.arg(flag);
        }
        cmd
    }

    pub fn run(&mut self) {
        if !self.should_rerun() {
            return;
        }
        let mut cmd = self.build_command();
        println!("[Linking]: {}", self.name);
        event!(Level::DEBUG, "executing: {:?}", cmd);
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        match cmd.output() {
            Ok(_) => {}
            Err(e) => {
                event!(Level::WARN, "error occured: `{}`", e);
            }
        }
    }
}
