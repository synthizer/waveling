use std::fmt::{Display, Formatter};

use mlua::Lua;

const UNKNOWN: &str = "<UNKNOWN>";

/// Effectively a backtrace of where a node or edge was declared in user code.
///
/// Frames are stored outermost first.
#[derive(Debug)]
pub struct SourceLoc {
    pub frames: Vec<SourceFrame>,
}

#[derive(Debug)]
pub struct SourceFrame {
    pub file: String,
    pub line: u32,
    pub function: String,
    pub printable_source: String,
}

impl Display for SourceLoc {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        for f in self.frames.iter() {
            writeln!(
                formatter,
                "{} line {}, in function {}",
                f.file, f.line, f.function
            )?;
            writeln!(formatter, "    {}", f.printable_source)?;
        }

        Ok(())
    }
}

impl SourceLoc {
    pub fn from_lua(l: &Lua) -> SourceLoc {
        let mut frames = vec![];

        for i in 0.. {
            if let Some(d) = l.inspect_stack(i) {
                let source = d.source();
                let printable_source =
                    String::from_utf8_lossy(source.short_src.unwrap_or(UNKNOWN.as_bytes()))
                        .into_owned();
                let line = d.curr_line() as u32;

                let maybe_file =
                    String::from_utf8_lossy(source.source.unwrap_or(UNKNOWN.as_bytes()));
                let file = if maybe_file.starts_with('@') {
                    maybe_file.strip_prefix('@').unwrap().to_string()
                } else if maybe_file.starts_with('=') {
                    maybe_file[1..maybe_file.len().min(50)].to_string()
                } else {
                    UNKNOWN.to_string()
                };

                let mut function =
                    String::from_utf8_lossy(d.names().name.unwrap_or(UNKNOWN.as_bytes()))
                        .into_owned();
                if function.is_empty() {
                    function = UNKNOWN.to_string();
                }

                frames.push(SourceFrame {
                    file,
                    line,
                    function,
                    printable_source,
                });
            }
        }

        // We have ended up with innermost to outermost, so flip it around.
        frames.reverse();

        SourceLoc { frames }
    }
}
