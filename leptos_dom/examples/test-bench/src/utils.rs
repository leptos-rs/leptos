pub struct MakeConsoleWriter;
use std::io::{self, Write};
use tracing_subscriber::fmt::MakeWriter;

impl<'a> MakeWriter<'a> for MakeConsoleWriter {
    type Writer = ConsoleWriter;

    fn make_writer(&'a self) -> Self::Writer {
        unimplemented!("use make_writer_for instead");
    }

    fn make_writer_for(&'a self, meta: &tracing::Metadata<'_>) -> Self::Writer {
        ConsoleWriter(*meta.level(), Vec::with_capacity(256))
    }
}

pub struct ConsoleWriter(tracing::Level, Vec<u8>);

impl io::Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.1.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        use gloo::console;
        use tracing::Level;

        let data = String::from_utf8(self.1.to_owned())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "data not UTF-8"))?;

        match self.0 {
            Level::TRACE => console::debug!(&data),
            Level::DEBUG => console::debug!(&data),
            Level::INFO => console::log!(&data),
            Level::WARN => console::warn!(&data),
            Level::ERROR => console::error!(&data),
        }

        Ok(())
    }
}

impl Drop for ConsoleWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
