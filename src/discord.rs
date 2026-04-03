use super::util::*;
use std::fmt::Write;

const DISCORD_MAX_MESSAGE_LENGTH: usize = 2000;
const DISCORD_TABLE_COL_SEP: usize = 4; //The number of whitespaces between 2 table columns

//A lazy output wrapper for sending discord messages
pub struct DiscordOutputWrapper {
    msg_buf: String,
    msg_empty: bool,
}

impl DiscordOutputWrapper {
    pub fn new() -> DiscordOutputWrapper {
        DiscordOutputWrapper {
            msg_buf: String::from("```"),
            msg_empty: true,
        }
    }
}

impl OutputWrapper for DiscordOutputWrapper {
    fn output(&mut self, msg: &impl std::fmt::Display) {
        std::write!(self.msg_buf, "{}", msg).unwrap();
        self.msg_empty = false;
    }
    fn output_line(&mut self, msg: &impl std::fmt::Display) {
        std::writeln!(self.msg_buf, "{}", msg).unwrap();
        self.msg_empty = false;
    }
    fn output_table(&mut self, table: &[Vec<String>]) {
        // IMRPOVE: Maybe use markdown formatting?
        let num_cols = table.iter().map(|row| row.len()).max().unwrap_or(0);
        let mut col_lengths: Vec<usize> = Vec::with_capacity(num_cols);
        for col in 0..num_cols {
            col_lengths.push(0);
            for row in table {
                col_lengths[col] = std::cmp::max(
                    col_lengths[col],
                    row.get(col).map_or(0, |s| s.len()) + DISCORD_TABLE_COL_SEP,
                );
            }
        }
        if let Some(col_length) = col_lengths.last_mut() {
            *col_length -= DISCORD_TABLE_COL_SEP; //Don't add spacing after the last column
        }

        for row in table {
            for (col, entry) in row.iter().enumerate() {
                self.msg_buf.push_str(entry);
                self.msg_buf
                    .extend(std::iter::repeat_n(' ', col_lengths[col] - entry.len()));
            }
            self.msg_buf.push('\n');
        }
        self.msg_empty = false;
    }
    fn new_line(&mut self) {
        self.msg_buf.push('\n');
    }
}
