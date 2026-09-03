// The kernel's lib/ctype.c table, which the discipline uses instead of ASCII or Unicode classes

/// Linux `iscntrl`, the C0 range and DEL
pub const fn is_cntrl(c: u8) -> bool {
    c < 0x20 || c == 0x7f
}

/// Linux `isalnum`, ASCII letters and digits plus the Latin-1 letters
pub const fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric() || is_upper(c) || is_lower(c)
}

/// Linux `tolower`, which adds 32 to every uppercase letter, Latin-1 included
pub const fn to_lower(c: u8) -> u8 {
    if is_upper(c) { c.wrapping_add(32) } else { c }
}

/// Linux `toupper`, which subtracts 32 from every lowercase letter, so 0xDF gives 0xBF
pub const fn to_upper(c: u8) -> u8 {
    if is_lower(c) { c.wrapping_sub(32) } else { c }
}

/// Linux `isupper`, ASCII and Latin-1 uppercase letters, where 0xD7 is the multiplication sign
const fn is_upper(c: u8) -> bool {
    c.is_ascii_uppercase() || (matches!(c, 0xC0..=0xDE) && c != 0xD7)
}

/// Linux `islower`, ASCII and Latin-1 lowercase letters, where 0xF7 is the division sign
const fn is_lower(c: u8) -> bool {
    c.is_ascii_lowercase() || (matches!(c, 0xDF..=0xFF) && c != 0xF7)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{is_alnum, is_cntrl, to_lower, to_upper};

    #[test]
    fn high_bytes_below_0xa0_are_not_control_characters() {
        assert_eq!(
            (is_cntrl(0x1f), is_cntrl(0x7f), is_cntrl(0x85)),
            (true, true, false)
        );
    }

    #[test]
    fn latin_letters_are_word_characters_and_the_signs_are_not() {
        assert_eq!(
            (
                is_alnum(0xC3),
                is_alnum(0xD7),
                is_alnum(0xF7),
                is_alnum(0xFF)
            ),
            (true, false, false, true)
        );
    }

    #[test]
    fn case_mapping_covers_latin_letters_only() {
        assert_eq!(
            (
                to_lower(0xC9),
                to_lower(0xD7),
                to_upper(0xE9),
                to_upper(0xF7)
            ),
            (0xE9, 0xD7, 0xC9, 0xF7)
        );
    }

    #[test]
    fn sharp_s_uppercases_by_the_kernel_offset() {
        assert_eq!((to_upper(0xDF), to_upper(0xFF)), (0xBF, 0xDF));
    }
}
