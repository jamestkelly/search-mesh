use std::io::{self, BufRead, Write};

use search_mesh_mcp::{ServerError, handle_jsonrpc};

fn main() -> Result<(), ServerError> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if let Some(response) = handle_jsonrpc(&line) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }

    Ok(())
}
