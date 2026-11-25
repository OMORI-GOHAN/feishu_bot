#[derive(Debug)]
pub struct ParsedCommand {
    pub name: String,
    pub args: String,
}

pub fn parse_command(text: &str) -> Option<ParsedCommand> {
    match text.trim_start().split_once(char::is_whitespace) {
        Some((_, remaining)) => {
            let remaining = remaining.trim_start();
            if remaining.is_empty() {
                return None;
            }
            match remaining.split_once(char::is_whitespace) {
                Some((command, args)) => {
                    let parsed_command = ParsedCommand {
                        name: command.to_string(),
                        args: args.trim_start().to_string(),
                    };
                    return Some(parsed_command);
                }
                None => {
                    let parsed_command = ParsedCommand {
                        name: remaining.to_string(),
                        args: "".to_string(),
                    };
                    return Some(parsed_command);
                }
            }
        }
        None => {}
    }
    None
}
