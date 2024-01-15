use std::fmt;
use std::io::{self, Write};
use std::sync::Mutex;

use ansi_term::Color;
use fern::{Dispatch, FormatCallback};
use lazy_static::lazy_static;
use log::{Level, LevelFilter, SetLoggerError};

lazy_static! {
    static ref GLOBAL_CONTEXT: Mutex<Option<String>> = Mutex::new(None);
    static ref OUTPUT_MUTEX: Mutex<()> = Mutex::new(());
}

pub struct GlobalContext {
}

impl GlobalContext {
    pub fn new(name: &str) -> GlobalContext {
        let context_string = format!("[{}] ", name);

        {
            let mut context = GLOBAL_CONTEXT.lock().unwrap();
            if context.is_some() {
                panic!("An attempt to set a nested global context");
            }
            *context = Some(context_string);
        }

        GlobalContext{}
    }

    fn get() -> String {
        GLOBAL_CONTEXT.lock().unwrap().as_ref().map(Clone::clone).unwrap_or_default()
    }
}

impl Drop for GlobalContext {
    fn drop(&mut self) {
        *GLOBAL_CONTEXT.lock().unwrap() = None;
    }
}

pub fn init(module_name: &'static str, level: Level) -> Result<(), SetLoggerError> {
    builder(module_name, level).apply()
}

pub fn builder(module_name: &'static str, level: Level) -> Dispatch {
    let debug_mode = level >= Level::Debug;

    let stdout_dispatcher =
        configure_formatter(Dispatch::new(), debug_mode, atty::is(atty::Stream::Stdout))
        .filter(|metadata| {metadata.level() >= Level::Info})
        .chain(io::stdout());

    let stderr_dispatcher =
        configure_formatter(Dispatch::new(), debug_mode, atty::is(atty::Stream::Stderr))
        .filter(|metadata| {metadata.level() < Level::Info})
        .chain(io::stderr());

    Dispatch::new()
        .level(if debug_mode {
            LevelFilter::Warn
        } else {
            LevelFilter::Off
        })
        .level_for(module_name, level.to_level_filter())
        .chain(stdout_dispatcher)
        .chain(stderr_dispatcher)
}

fn configure_formatter(dispatcher: Dispatch, debug_mode: bool, colored_output: bool) -> Dispatch {
    if debug_mode {
        dispatcher.format(move |out, message, record| {
            let level = record.level();
            let level_name = get_level_name(level);
            let time = chrono::Local::now().format("[%T%.3f]");

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
                let level_color = get_level_color(level);
                write_log(out, level, format_args!(
                    "{color_prefix}{time}{file} {level}: {context}{message}{color_suffix}",
                    color_prefix=level_color.prefix(), time=time, file=file, level=level_name,
                    context=GlobalContext::get(), message=message, color_suffix=level_color.suffix()
                ));
            } else {
                write_log(out, level, format_args!(
                    "{time}{file} {level}: {context}{message}",
                    time=time, file=file, level=level_name, context=GlobalContext::get(),
                    message=message
                ));
            }
        })
    } else {
        dispatcher.format(move |out, message, record| {
            let level = record.level();
            let level_name = get_level_name(level);

            if colored_output {
                let level_color = get_level_color(level);
                write_log(out, level, format_args!(
                    "{color_prefix}{level}: {context}{message}{color_suffix}",
                    color_prefix=level_color.prefix(), level=level_name,
                    context=GlobalContext::get(), message=message, color_suffix=level_color.suffix()
                ));
            } else {
                write_log(out, level, format_args!("{level}: {context}{message}",
                    level=level_name, context=GlobalContext::get(), message=message));
            }
        })
    }
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

fn get_level_name(level: Level) -> &'static str {
    match level {
        Level::Error => "E",
        Level::Warn  => "W",
        Level::Info  => "I",
        Level::Debug => "D",
        Level::Trace => "T",
    }
}

fn write_log(out: FormatCallback, level: Level, formatted_message: fmt::Arguments) {
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