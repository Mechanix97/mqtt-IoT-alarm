pub mod alarm;
pub mod constants;
pub mod telegram;

pub fn parse_on_off(input: &[u8]) -> bool {
    match input {
        b"ON" => true,
        _ => false,
    }
}
