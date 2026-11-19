//! Emojis d'avatar (mêmes que la version web) : image Twemoji (CC-BY 4.0) embarquée pour chacun.

pub static EMOJIS: [(&str, &[u8]); 55] = [
    ("⚓", include_bytes!("../assets/emoji/00.png")),
    ("🚢", include_bytes!("../assets/emoji/01.png")),
    ("⛵", include_bytes!("../assets/emoji/02.png")),
    ("🛥️", include_bytes!("../assets/emoji/03.png")),
    ("🐙", include_bytes!("../assets/emoji/04.png")),
    ("🦈", include_bytes!("../assets/emoji/05.png")),
    ("🐬", include_bytes!("../assets/emoji/06.png")),
    ("🐳", include_bytes!("../assets/emoji/07.png")),
    ("🐋", include_bytes!("../assets/emoji/08.png")),
    ("🐟", include_bytes!("../assets/emoji/09.png")),
    ("🐠", include_bytes!("../assets/emoji/10.png")),
    ("🐡", include_bytes!("../assets/emoji/11.png")),
    ("🦀", include_bytes!("../assets/emoji/12.png")),
    ("🦞", include_bytes!("../assets/emoji/13.png")),
    ("🐢", include_bytes!("../assets/emoji/14.png")),
    ("🐧", include_bytes!("../assets/emoji/15.png")),
    ("🦅", include_bytes!("../assets/emoji/16.png")),
    ("🦉", include_bytes!("../assets/emoji/17.png")),
    ("🦜", include_bytes!("../assets/emoji/18.png")),
    ("🐺", include_bytes!("../assets/emoji/19.png")),
    ("🦊", include_bytes!("../assets/emoji/20.png")),
    ("🐻", include_bytes!("../assets/emoji/21.png")),
    ("🐼", include_bytes!("../assets/emoji/22.png")),
    ("🦁", include_bytes!("../assets/emoji/23.png")),
    ("🐯", include_bytes!("../assets/emoji/24.png")),
    ("🐸", include_bytes!("../assets/emoji/25.png")),
    ("🐵", include_bytes!("../assets/emoji/26.png")),
    ("🦄", include_bytes!("../assets/emoji/27.png")),
    ("🐲", include_bytes!("../assets/emoji/28.png")),
    ("🌊", include_bytes!("../assets/emoji/29.png")),
    ("🏝️", include_bytes!("../assets/emoji/30.png")),
    ("🌴", include_bytes!("../assets/emoji/31.png")),
    ("☀️", include_bytes!("../assets/emoji/32.png")),
    ("🌙", include_bytes!("../assets/emoji/33.png")),
    ("⭐", include_bytes!("../assets/emoji/34.png")),
    ("🚀", include_bytes!("../assets/emoji/35.png")),
    ("🛸", include_bytes!("../assets/emoji/36.png")),
    ("🔥", include_bytes!("../assets/emoji/37.png")),
    ("⚡", include_bytes!("../assets/emoji/38.png")),
    ("🌈", include_bytes!("../assets/emoji/39.png")),
    ("🍀", include_bytes!("../assets/emoji/40.png")),
    ("🎯", include_bytes!("../assets/emoji/41.png")),
    ("🏆", include_bytes!("../assets/emoji/42.png")),
    ("👑", include_bytes!("../assets/emoji/43.png")),
    ("🎮", include_bytes!("../assets/emoji/44.png")),
    ("🎲", include_bytes!("../assets/emoji/45.png")),
    ("🧭", include_bytes!("../assets/emoji/46.png")),
    ("🔭", include_bytes!("../assets/emoji/47.png")),
    ("🤖", include_bytes!("../assets/emoji/48.png")),
    ("👾", include_bytes!("../assets/emoji/49.png")),
    ("😀", include_bytes!("../assets/emoji/50.png")),
    ("😎", include_bytes!("../assets/emoji/51.png")),
    ("🤠", include_bytes!("../assets/emoji/52.png")),
    ("🥳", include_bytes!("../assets/emoji/53.png")),
    ("😺", include_bytes!("../assets/emoji/54.png")),
];

pub fn valid(e: &str) -> bool {
    EMOJIS.iter().any(|(x, _)| *x == e)
}

pub fn bytes(e: &str) -> Option<&'static [u8]> {
    EMOJIS.iter().find(|(x, _)| *x == e).map(|(_, b)| *b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_emoji_decodes_and_matches_the_web_list() {
        assert_eq!(EMOJIS.len(), 55);
        for (e, b) in EMOJIS.iter() {
            let img = image::load_from_memory(b).unwrap_or_else(|_| panic!("{e} illisible")).to_rgba8();
            assert_eq!(img.width(), 72);
            assert!(crate::accounts::valid_photo(&format!("emoji:{e}")));
            assert!(crate::accounts::decode_photo(&format!("emoji:{e}")).is_some());
        }
        assert!(!crate::accounts::valid_photo("emoji:💣"));
    }
}
