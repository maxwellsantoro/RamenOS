//! One bounded stdin/stdout transport for both typed arms.
use crate::protocol::{Bootstrap, MAX_REQUEST_BYTES};
use std::io::{self, BufRead, Read, Write};
pub fn serve(
    bootstrap: &Bootstrap,
    mut execute: impl FnMut(&[u8]) -> io::Result<Vec<u8>>,
) -> io::Result<()> {
    let mut out = io::stdout().lock();
    out.write_all(&serde_json::to_vec(bootstrap)?)?;
    out.write_all(b"\n")?;
    out.flush()?;
    let mut input = io::stdin().lock();
    loop {
        let mut line = Vec::new();
        let count = (&mut input)
            .take((MAX_REQUEST_BYTES + 2) as u64)
            .read_until(b'\n', &mut line)?;
        if count == 0 {
            break;
        }
        if line.ends_with(b"\n") {
            line.pop();
        }
        if line.len() > MAX_REQUEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request frame limit",
            ));
        }
        out.write_all(&execute(&line)?)?;
        out.write_all(b"\n")?;
        out.flush()?;
    }
    Ok(())
}
