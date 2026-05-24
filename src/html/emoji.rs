use std::str::FromStr;

use takumi::layout::{
    node::Node,
    style::{Style, StyleDeclarationBlock},
};
use unicode_segmentation::UnicodeSegmentation;

use super::{HtmlError, Result};

const ZERO_WIDTH_JOINER: char = '\u{200D}';
const VARIATION_SELECTOR_16: char = '\u{FE0F}';
const KEYCAP_MARK: char = '\u{20E3}';
const EMOJI_IMAGE_STYLE: &str = "display: inline-block; width: 1em; height: 1em; margin: 0 0.05em 0 0.1em; vertical-align: -0.1em;";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmojiType {
    Twemoji,
    Blobmoji,
    Noto,
    Openmoji,
    Fluent,
    FluentFlat,
}

impl EmojiType {
    fn image_url(self, icon: &str) -> String {
        let code = emoji_icon_code(icon);

        match self {
            Self::Twemoji => format!(
                "https://cdn.jsdelivr.net/gh/jdecked/twemoji@17.0.2/assets/svg/{}.svg",
                code.to_ascii_lowercase()
            ),
            Self::Openmoji => format!(
                "https://cdn.jsdelivr.net/npm/@svgmoji/openmoji@2.0.0/svg/{}.svg",
                code.to_ascii_uppercase()
            ),
            Self::Blobmoji => format!(
                "https://cdn.jsdelivr.net/npm/@svgmoji/blob@2.0.0/svg/{}.svg",
                code.to_ascii_uppercase()
            ),
            Self::Noto => format!(
                "https://cdn.jsdelivr.net/gh/googlefonts/noto-emoji@v2.051/svg/emoji_u{}.svg",
                code.to_ascii_lowercase().replace('-', "_")
            ),
            Self::Fluent => format!(
                "https://cdn.jsdelivr.net/gh/shuding/fluentui-emoji-unicode/assets/{}_color.svg",
                code.to_ascii_lowercase()
            ),
            Self::FluentFlat => format!(
                "https://cdn.jsdelivr.net/gh/shuding/fluentui-emoji-unicode/assets/{}_flat.svg",
                code.to_ascii_lowercase()
            ),
        }
    }
}

pub(crate) fn split_text_to_nodes(text: &str, emoji_type: EmojiType) -> Result<Option<Vec<Node>>> {
    let mut nodes = Vec::new();
    let mut current_text = String::new();
    let mut found_emoji = false;

    for segment in text.graphemes(true) {
        if is_emoji_segment(segment) {
            found_emoji = true;
            if !current_text.is_empty() {
                nodes.push(Node::text(std::mem::take(&mut current_text)));
            }
            nodes.push(emoji_image_node(segment, emoji_type)?);
        } else {
            current_text.push_str(segment);
        }
    }

    if !current_text.is_empty() {
        nodes.push(Node::text(current_text));
    }

    Ok(found_emoji.then_some(nodes))
}

fn emoji_image_node(segment: &str, emoji_type: EmojiType) -> Result<Node> {
    Ok(Node::image((emoji_type.image_url(segment), None, None)).with_style(emoji_image_style()?))
}

fn emoji_image_style() -> Result<Style> {
    StyleDeclarationBlock::from_str(EMOJI_IMAGE_STYLE)
        .map(Into::into)
        .map_err(|error| HtmlError::InlineStyleParse {
            tag_name: "img".to_owned(),
            value: EMOJI_IMAGE_STYLE.to_owned(),
            reason: error.to_string(),
        })
}

fn emoji_icon_code(segment: &str) -> String {
    let normalized = if segment.contains(ZERO_WIDTH_JOINER) {
        segment.to_owned()
    } else {
        segment
            .chars()
            .filter(|ch| *ch != VARIATION_SELECTOR_16)
            .collect::<String>()
    };

    normalized
        .chars()
        .map(|ch| format!("{:x}", ch as u32))
        .collect::<Vec<_>>()
        .join("-")
}

fn is_emoji_segment(segment: &str) -> bool {
    is_keycap_emoji(segment)
        || is_regional_indicator_pair(segment)
        || segment.chars().any(is_extended_pictographic)
}

fn is_keycap_emoji(segment: &str) -> bool {
    let chars = segment.chars().collect::<Vec<_>>();
    match chars.as_slice() {
        [base, KEYCAP_MARK] => is_keycap_base(*base),
        [base, VARIATION_SELECTOR_16, KEYCAP_MARK] => is_keycap_base(*base),
        _ => false,
    }
}

fn is_keycap_base(ch: char) -> bool {
    matches!(ch, '#' | '*' | '0'..='9')
}

fn is_regional_indicator_pair(segment: &str) -> bool {
    let chars = segment.chars().collect::<Vec<_>>();
    chars.len() == 2 && chars.into_iter().all(is_regional_indicator)
}

fn is_regional_indicator(ch: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&ch)
}

fn is_extended_pictographic(ch: char) -> bool {
    let code = ch as u32;

    matches!(
        code,
        0x00A9
            | 0x00AE
            | 0x203C
            | 0x2049
            | 0x2122
            | 0x2139
            | 0x231A
            | 0x231B
            | 0x2328
            | 0x23CF
            | 0x23E9
            | 0x23EA
            | 0x23EB
            | 0x23EC
            | 0x23ED
            | 0x23EE
            | 0x23EF
            | 0x23F0
            | 0x23F1
            | 0x23F2
            | 0x23F3
            | 0x23F8
            | 0x23F9
            | 0x23FA
            | 0x24C2
            | 0x25AA
            | 0x25AB
            | 0x25B6
            | 0x25C0
            | 0x25FB
            | 0x25FC
            | 0x25FD
            | 0x25FE
            | 0x2600
            | 0x2601
            | 0x2602
            | 0x2603
            | 0x2604
            | 0x2605
            | 0x2607
            | 0x2608
            | 0x2609
            | 0x260A
            | 0x260B
            | 0x260C
            | 0x260D
            | 0x260E
            | 0x260F
            | 0x2611
            | 0x2614
            | 0x2615
            | 0x2618
            | 0x261D
            | 0x2620
            | 0x2622
            | 0x2623
            | 0x2626
            | 0x262A
            | 0x262E
            | 0x262F
            | 0x2638
            | 0x2639
            | 0x263A
            | 0x2640
            | 0x2642
            | 0x2648
            | 0x2649
            | 0x264A
            | 0x264B
            | 0x264C
            | 0x264D
            | 0x264E
            | 0x264F
            | 0x2650
            | 0x2651
            | 0x2652
            | 0x2653
            | 0x265F
            | 0x2660
            | 0x2663
            | 0x2665
            | 0x2666
            | 0x2668
            | 0x267B
            | 0x267E
            | 0x267F
            | 0x2692
            | 0x2693
            | 0x2694
            | 0x2695
            | 0x2696
            | 0x2697
            | 0x2699
            | 0x269B
            | 0x269C
            | 0x26A0
            | 0x26A1
            | 0x26A7
            | 0x26AA
            | 0x26AB
            | 0x26B0
            | 0x26B1
            | 0x26BD
            | 0x26BE
            | 0x26C4
            | 0x26C5
            | 0x26C8
            | 0x26CE
            | 0x26CF
            | 0x26D1
            | 0x26D3
            | 0x26D4
            | 0x26E9
            | 0x26EA
            | 0x26F0
            | 0x26F1
            | 0x26F2
            | 0x26F3
            | 0x26F4
            | 0x26F5
            | 0x26F7
            | 0x26F8
            | 0x26F9
            | 0x26FA
            | 0x26FD
            | 0x2702
            | 0x2705
            | 0x2708
            | 0x2709
            | 0x270A
            | 0x270B
            | 0x270C
            | 0x270D
            | 0x270F
            | 0x2712
            | 0x2714
            | 0x2716
            | 0x271D
            | 0x2721
            | 0x2728
            | 0x2733
            | 0x2734
            | 0x2744
            | 0x2747
            | 0x274C
            | 0x274E
            | 0x2753
            | 0x2754
            | 0x2755
            | 0x2757
            | 0x2763
            | 0x2764
            | 0x27A1
            | 0x2934
            | 0x2935
            | 0x2B05
            | 0x2B06
            | 0x2B07
            | 0x2B1B
            | 0x2B1C
            | 0x2B50
            | 0x2B55
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
    ) || (0x1F000..=0x1FAFF).contains(&code)
}
