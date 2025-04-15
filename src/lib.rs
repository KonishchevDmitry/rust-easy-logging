mod context;

use std::fmt;
use std::io::{self, Write};
use std::sync::Mutex;

use ansi_term::Color;
use fern::{Dispatch, FormatCallback};
use lazy_static::lazy_static;
use log::{Level, LevelFilter, SetLoggerError};

pub use fern;
pub use log;
pub use crate::context::GlobalContext;

pub struct LoggingConfig {
    minimal: bool,
    modules: Vec<ModuleConfig>,
    get_level_name: fn (level: Level) -> &'static str,
}

impl LoggingConfig {
    pub fn new(main_module_name: &'static str, level: Level) -> Self {
        LoggingConfig {
            minimal: false,
            modules: vec![ModuleConfig {
                name: main_module_name,
                level,
            }],
            get_level_name: |level| {
                match level {
                    Level::Error => "E: ",
                    Level::Warn  => "W: ",
                    Level::Info  => "I: ",
                    Level::Debug => "D: ",
                    Level::Trace => "T: ",
                }
            }
        }
    }

    pub fn level_for(mut self, name: &'static str, level: Level) -> Self {
        self.modules.push(ModuleConfig {name, level});
        self
    }

    pub fn level_names(mut self, get: fn (level: Level) -> &'static str) -> Self {
        self.get_level_name = get;
        self
    }

    pub fn minimal(mut self) -> Self {
        self.minimal = true;
        self
    }

    pub fn dispatch(self) -> Dispatch {
        let stdout_dispatcher =
            self.configure_formatter(Dispatch::new(), atty::is(atty::Stream::Stdout))
            .filter(|metadata| metadata.level() >= Level::Info)
            .chain(io::stdout());

        let stderr_dispatcher =
            self.configure_formatter(Dispatch::new(), atty::is(atty::Stream::Stderr))
            .filter(|metadata| metadata.level() < Level::Info)
            .chain(io::stderr());

        let base_level = self.modules.first().unwrap().level;
        let mut dispatch = Dispatch::new()
            .level(if base_level >= Level::Debug {
                LevelFilter::Warn
            } else {
                LevelFilter::Off
            })
            .chain(stdout_dispatcher)
            .chain(stderr_dispatcher);

        for module in self.modules {
            dispatch = dispatch.level_for(module.name, module.level.to_level_filter());
        }

        dispatch
    }

    pub fn build(self) -> Result<(), SetLoggerError> {
        self.dispatch().apply()
    }

    fn configure_formatter(&self, dispatcher: Dispatch, colored_output: bool) -> Dispatch {
        let base_level = self.modules.first().unwrap().level;

        let mut get_level_name = self.get_level_name;
        if self.minimal && base_level < Level::Debug {
            get_level_name = |_: Level| "";
        }

        if base_level < Level::Debug {
            dispatcher.format(move |out, message, record| {
                let level = record.level();
                let level_name = get_level_name(level);
                let context = GlobalContext::get(base_level);

                if colored_output {
                    let color = get_level_color(level);
                    write_log(out, level, format_args!(
                        "{color_prefix}{level_name}{context}{message}{color_suffix}",
                        color_prefix=color.prefix(), color_suffix=color.suffix(),
                    ));
                } else {
                    write_log(out, level, format_args!("{level_name}{context}{message}"));
                }
            })
        } else {
            dispatcher.format(move |out, message, record| {
                let time = chrono::Local::now().format("[%T%.3f]");
                let level = record.level();
                let level_name = get_level_name(level);
                let context = GlobalContext::get(base_level);

                let file = if let (Some(mut file), Some(line)) = (record.file(), record.line()) {
                    let mut file_width = 10;
                    let mut line_width = 3;
                    let mut line_extra_width = line / 1000;

                    while line_extra_width > 0 && file_width > 0 {
                        line_width += 1;
                        file_width -= 1;
                        line_extra_width /= 10;
                    }

                    if file.starts_with("src/") {
                        file = &file[4..];
                    }

                    if file.len() > file_width {
                        file = &file[file.len() - file_width..]
                    }

                    format!(" [{file:>file_width$}:{line:0line_width$}]",
                            file=file, file_width=file_width, line=line, line_width=line_width)
                } else {
                    String::new()
                };

                if colored_output {
                    let color = get_level_color(level);
                    write_log(out, level, format_args!(
                        "{color_prefix}{time}{file} {level_name}{context}{message}{color_suffix}",
                        color_prefix=color.prefix(), color_suffix=color.suffix()
                    ));
                } else {
                    write_log(out, level, format_args!("{time}{file} {level_name}{context}{message}"));
                }
            })
        }
    }
}

pub fn init(main_module_name: &'static str, level: Level) -> Result<(), SetLoggerError> {
    LoggingConfig::new(main_module_name, level).build()
}

struct ModuleConfig {
    name: &'static str,
    level: Level,
}

fn get_level_color(level: Level) -> Color {
    match level {
        Level::Error => Color::Red,
        Level::Warn  => Color::Yellow,
        Level::Info  => Color::Green,
        Level::Debug => Color::Cyan,
        Level::Trace => Color::Purple,
    }
}

fn write_log(out: FormatCallback, level: Level, formatted_message: fmt::Arguments) {
    lazy_static! {
        static ref OUTPUT_MUTEX: Mutex<()> = Mutex::new(());
    }

    // Since we write into stdout and stderr we should guard any write with a mutex to not get the
    // output interleaved.
    let _lock = OUTPUT_MUTEX.lock();

    out.finish(formatted_message);

    let _ = if level >= Level::Info {
        io::stdout().flush()
    } else {
        io::stderr().flush()
    };
}