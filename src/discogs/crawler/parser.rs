use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde::{Deserialize, Serialize};

const TRACK_SIDE_REGEX: &str = r#"^[a-zA-Z][1-9]$"#;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub artists: Vec<String>,
    pub catno: String,
    pub remix: Vec<String>,
    pub position: String,
}

impl Track {
    pub fn artist_name(&self, separator: Option<&str>) -> String {
        let sep = separator.unwrap_or(" ");
        self.artists.to_vec().join(sep)
    }

    pub fn remixer_name(&self, separator: Option<&str>) -> String {
        let sep = separator.unwrap_or(" ");
        self.remix.to_vec().join(sep)
    }

    pub fn from_str(str: &str) -> Self {
        parse_track(str)
    }
}

impl PartialEq for Track {
    fn eq(&self, other: &Self) -> bool {
        if self.artists.len() != other.artists.len() {
            return false;
        }

        if self.remix.len() != other.remix.len() {
            return false;
        }

        if !self
            .name
            .to_ascii_lowercase()
            .eq(&other.name.to_ascii_lowercase())
        {
            return false;
        }

        for artist in &self.artists {
            let mut found = false;
            for o_artist in &other.artists {
                if o_artist
                    .to_ascii_lowercase()
                    .eq(&artist.to_ascii_lowercase())
                {
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }

        for r in &self.remix {
            let mut found = false;
            for o_remix in &other.remix {
                if o_remix.to_ascii_lowercase().eq(&r.to_ascii_lowercase()) {
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenType {
    And,
    Column,
    LeftBracket,
    LeftParen,
    Literal,
    Minus,
    Plus,
    Feat,
    RightBracket,
    RightParen,
    Mix,
    EOF,
    Dot,
}

#[derive(Debug, Clone)]
struct Token {
    lexeme: Option<String>,
    token_type: TokenType,
}

struct Scanner {
    input: String,
    chars: Vec<char>,
    current: usize,
    end: usize,
    start: usize,
    tokens: Vec<Token>,
    track_side_matcher: Regex,
    keywords: HashMap<String, TokenType>,
}

impl Scanner {
    pub fn new(input: String) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let end = chars.len();

        Scanner {
            input,
            chars,
            current: 0,
            end,
            start: 0,
            tokens: vec![],
            track_side_matcher: Regex::new(TRACK_SIDE_REGEX).unwrap(),
            keywords: HashMap::from_iter([
                ("remix".into(), TokenType::Mix),
                ("mix".into(), TokenType::Mix),
                ("instrumental".into(), TokenType::Mix),
                ("instr.".into(), TokenType::Mix),
                ("feat".into(), TokenType::Feat),
                ("feat.".into(), TokenType::Feat),
                ("Remix".into(), TokenType::Mix),
                ("Mix".into(), TokenType::Mix),
                ("reconstruction".into(), TokenType::Mix),
                ("Reconstruction".into(), TokenType::Mix),
            ]),
        }
    }
    pub fn scan(&mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.add_token(TokenType::EOF, None);
        self.tokens.to_vec()
    }

    fn scan_token(&mut self) {
        let ch = self.advance();

        match self.is_special_token(ch) {
            Some(v) => self.add_token(v, None),
            None => match ch {
                ' ' | '\t' => {}
                _ => {
                    let maybe_kw = self.keyword();
                    if let Some(keyword) = &maybe_kw {
                        match self.keywords.get(keyword.as_str()) {
                            Some(v) => self.add_token(*v, None),
                            None => self.add_token(TokenType::Literal, maybe_kw.clone()),
                        }
                    }
                }
            },
        };
    }

    fn is_special_token(&self, ch: char) -> Option<TokenType> {
        match ch {
            '(' => Some(TokenType::LeftParen),
            ')' => Some(TokenType::RightParen),
            '[' => Some(TokenType::LeftBracket),
            ']' => Some(TokenType::RightBracket),
            '&' => Some(TokenType::And),
            ',' => Some(TokenType::And),
            '-' => Some(TokenType::Minus),
            '+' => Some(TokenType::Plus),
            ':' => Some(TokenType::Column),
            '.' => Some(TokenType::Dot),
            '\0' => Some(TokenType::EOF),
            _ => None,
        }
    }

    fn is_track_side(&self, value: &str) -> bool {
        self.track_side_matcher.is_match(value)
    }

    fn add_token(&mut self, token_type: TokenType, lexeme: Option<String>) {
        let token = Token {
            token_type,
            lexeme: if lexeme.is_some() { lexeme } else { None },
        };

        self.tokens.push(token);
    }

    fn advance(&mut self) -> char {
        let ch = self.chars[self.current];
        self.current += 1;
        ch
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.end
    }

    fn is_whitespace(&self, ch: char) -> bool {
        ch == ' '
    }

    fn keyword(&mut self) -> Option<String> {
        loop {
            let next_char = self.peek();
            let special_token = self.is_special_token(next_char);
            if special_token.is_some() || self.is_whitespace(next_char) {
                match special_token {
                    Some(TokenType::Dot) => {
                        let part = self.input.get(self.start..self.current).unwrap();
                        if self.is_track_side(part) {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            self.advance();
        }

        if self.start != self.current {
            Some(
                self.input
                    .get(self.start..self.current)
                    .unwrap()
                    .to_string(),
            )
        } else {
            None
        }
    }

    fn peek(&self) -> char {
        match self.is_at_end() {
            true => '\0',
            false => self.chars[self.current],
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
enum TrackPart {
    ArtistName(String),
    Label(String),
    Remix(String),
    TrackName(String),
    TrackSide(String),
}

struct Parser {
    current: usize,
    tokens: Vec<Token>,
    track_side_matcher: Regex,
    pub track_parts: Vec<TrackPart>,
}

impl Parser {
    pub fn new(input: String) -> Self {
        let mut scanner = Scanner::new(input);
        let tokens = scanner.scan();

        Parser {
            current: 0,
            tokens,
            track_side_matcher: Regex::new(TRACK_SIDE_REGEX).unwrap(),
            track_parts: vec![],
        }
    }
    pub fn parse(&mut self) -> Vec<TrackPart> {
        while !self.is_at_end() {
            let part = self.consume_literals();
            match self.peek().token_type {
                TokenType::Dot | TokenType::Column => {
                    if self.is_track_side(&part) {
                        self.add_part(TrackPart::TrackSide(part))
                    }
                }
                TokenType::Minus => {
                    if let Some(token) = self.peek_before() {
                        if token.token_type.eq(&TokenType::Mix) {
                            self.advance();
                            let track_name = self.consume_literals();
                            self.add_part(TrackPart::TrackName(track_name));
                        } else if !part.is_empty() {
                            self.add_part(TrackPart::ArtistName(part));
                        }
                    }
                }
                TokenType::And | TokenType::Feat => {
                    if self.is_remixer() {
                        self.add_part(TrackPart::Remix(part));
                    } else {
                        self.add_part(TrackPart::ArtistName(part));
                    }
                    self.advance();
                    let second_artist_name = self.consume_literals();
                    if self.is_remixer() {
                        self.add_part(TrackPart::Remix(second_artist_name));
                        self.advance();
                    } else {
                        self.add_part(TrackPart::ArtistName(second_artist_name));
                    }
                    continue;
                }
                TokenType::LeftParen => {
                    // If we find a left parenthesis, we need to check if it's at the beginning
                    // of a track name (after a hyphen)
                    if part.is_empty() && !self.track_parts.is_empty() {
                        // This means we likely have a track name starting with parenthesis
                        self.advance(); // Consume the left paren

                        // Collect content inside parentheses
                        let inside_paren = self.consume_literals();

                        // Build the track name starting with the opening parenthesis
                        let mut track_name = String::from("(");
                        track_name.push_str(&inside_paren);

                        // Check if we have a right paren
                        if self.peek().token_type == TokenType::RightParen {
                            track_name.push(')');
                            self.advance(); // Consume the right paren

                            // Now we need to collect any content after the closing parenthesis
                            // We'll collect all literals until the next special token or EOF
                            let mut collecting = true;
                            while collecting && !self.is_at_end() {
                                if self.peek().token_type == TokenType::Literal {
                                    let after_paren = self.consume_literals();
                                    track_name.push_str(&after_paren);
                                } else {
                                    // Stop if we see anything other than a literal
                                    collecting = false;
                                }
                            }

                            self.add_part(TrackPart::TrackName(track_name));
                        } else {
                            // If no right paren, just add what we have
                            self.add_part(TrackPart::TrackName(track_name));
                        }
                    } else {
                        self.add_part(TrackPart::TrackName(part));
                    }
                }
                TokenType::Mix => self.add_part(TrackPart::Remix(part)),
                TokenType::LeftBracket => {
                    if !self.track_parts.is_empty() {
                        let last_part = self.track_parts.last().unwrap();
                        match last_part {
                            TrackPart::Remix(_v) => {}
                            TrackPart::ArtistName(_v) => self.add_part(TrackPart::TrackName(part)),
                            _ => {}
                        }
                    }
                }
                TokenType::RightParen => {}
                TokenType::RightBracket => self.add_part(TrackPart::Label(part)),
                _ => self.add_part(TrackPart::TrackName(part)),
            }
            self.advance();
        }
        self.track_parts.to_vec()
    }

    fn is_track_side(&self, value: &str) -> bool {
        self.track_side_matcher.is_match(value)
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::EOF)
    }

    fn advance(&mut self) -> &Token {
        if self.is_at_end() {
            return self.tokens.last().unwrap();
        }
        let token = self.tokens.get(self.current);
        self.current += 1;
        token.unwrap()
    }

    fn consume_literals(&mut self) -> String {
        let mut consumed: Vec<String> = vec![];
        loop {
            if self.is_at_end() || !TokenType::Literal.eq(&self.peek().token_type) {
                break;
            }
            let token = self.advance();
            consumed.push(token.lexeme.as_ref().unwrap().to_owned());
        }

        consumed.join(" ")
    }

    fn add_part(&mut self, part_type: TrackPart) {
        self.track_parts.push(part_type);
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.current).unwrap()
    }

    fn peek_before(&self) -> Option<&Token> {
        match self.current {
            0 => None,
            _ => self.tokens.get(self.current - 1),
        }
    }

    fn is_remixer(&self) -> bool {
        let mut left_parens_found = false;
        let mut remix_token_found = false;

        for i in (0..=self.current).rev() {
            let current = self.tokens.get(i).unwrap();
            match current.token_type {
                TokenType::LeftParen => {
                    left_parens_found = true;
                    break;
                }
                TokenType::Minus => {
                    break;
                }
                _ => continue,
            }
        }

        for i in self.current..self.tokens.len() {
            let current = self.tokens.get(i).unwrap();
            match current.token_type {
                TokenType::Mix => {
                    remix_token_found = true;
                    break;
                }
                TokenType::RightParen => {
                    break;
                }
                _ => continue,
            }
        }
        left_parens_found && remix_token_found
    }
}

pub fn parse_track(input: &str) -> Track {
    let track_parts = Parser::new(input.replace(|c: char| !c.is_ascii(), "")).parse();
    let mut artists: HashSet<String> = HashSet::new();
    let mut name = String::new();
    let mut remix: HashSet<String> = HashSet::new();
    let mut position = String::new();
    let mut catno = String::new();

    for part in track_parts {
        match part {
            TrackPart::ArtistName(v) => {
                artists.insert(v);
            }
            TrackPart::Remix(v) => {
                if v.to_ascii_lowercase().eq("original") {
                    continue;
                }
                remix.insert(v);
            }
            TrackPart::TrackName(v) => {
                name = v;
            }
            TrackPart::TrackSide(v) => {
                position = v;
            }
            TrackPart::Label(v) => {
                catno = v;
            }
        };
    }

    Track {
        name,
        artists: Vec::from_iter(artists),
        remix: Vec::from_iter(remix),
        position,
        catno,
    }
}

#[test]
fn scanner_test() {
    println!(
        "{:?}",
        Scanner::new("Barac - Marea Neagra".to_string()).scan()
    );
    println!(
        "{:?}",
        Scanner::new("A1: Barac - Marea Neagra".to_string()).scan()
    );
    println!(
        "{:?}",
        Scanner::new("A1: Barac & Sublee - Marea Neagra (uu remix)".to_string()).scan()
    );
}

#[test]
fn track_side_test() {
    let track_parts = Parser::new("A1: foo".to_string()).parse();
    assert_eq!(
        track_parts,
        vec![
            TrackPart::TrackSide("A1".to_string()),
            TrackPart::TrackName("foo".to_string())
        ]
    );
}

#[cfg(test)]
mod tests {
    use super::parse_track;

    #[test]
    fn youtube_generic_title() {
        let track = parse_track("Unknown Artist - Untitled [EEE008]");
        assert_eq!(track.name, "Untitled");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists[0], "Unknown Artist");
        assert_eq!(track.catno, "EEE008");
    }

    #[test]
    fn youtube_generic_title2() {
        let track = parse_track("Ted Amber - Terula (Barut Remix) [BM006]");
        assert_eq!(track.name, "Terula");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists[0], "Ted Amber");
        assert_eq!(track.remix[0], "Barut");
        assert_eq!(track.catno, "BM006");
    }

    #[test]
    fn youtube_generic_title3() {
        let track = parse_track("A1. Vern - Eter [LKMV004]");
        assert_eq!(track.name, "Eter");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists[0], "Vern");
        assert_eq!(track.remix.len(), 0);
        assert_eq!(track.catno, "LKMV004");
        assert_eq!(track.position, "A1");
    }

    #[test]
    fn artist_with_dot() {
        let track = parse_track("Tobias. - Dial");
        assert_eq!(track.name, "Dial");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists[0], "Tobias.");
        assert_eq!(track.remix.len(), 0);
    }

    #[test]
    fn parantheses_in_track_name() {
        let track = parse_track("Rainfield - (Un)Respire");
        assert_eq!(track.name, "(Un)Respire");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists[0], "Rainfield");
    }

    #[test]
    fn multiple_remixers() {
        let mut track =
            parse_track("Mihai Popoviciu - Data On Data (Peter Makto & Gregory S Remix)");
        assert_eq!(track.name, "Data On Data");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists[0], "Mihai Popoviciu");
        assert_eq!(track.remix.len(), 2);

        track.remix.sort();
        assert_eq!(track.remix, ["Gregory S", "Peter Makto"]);
    }

    #[test]
    fn double_word_remixer() {
        let track = parse_track("Egal 3 - Play You (Povestea Continua Mix)");
        assert_eq!(track.name, "Play You");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists[0], "Egal 3");
        assert_eq!(track.remix.len(), 1);
        assert_eq!(track.remix, ["Povestea Continua"]);
    }

    #[test]
    fn double_artist() {
        let mut track = parse_track("Gorbani & Enzo Leep - L.E.M (Melodie Remix)");
        assert_eq!(track.name, "L.E.M");
        assert_eq!(track.artists.len(), 2);
        track.artists.sort();
        assert_eq!(track.artists, ["Enzo Leep", "Gorbani"]);
        assert_eq!(track.remix.len(), 1);
        assert_eq!(track.remix, vec!["Melodie"]);
    }

    #[test]
    fn original_mix() {
        let mut track = parse_track("Sonartek & Gorbani - Otherside (Original Mix)");
        assert_eq!(track.name, "Otherside");
        assert_eq!(track.artists.len(), 2);
        track.artists.sort();

        assert_eq!(track.artists, ["Gorbani", "Sonartek"]);
        assert_eq!(track.remix.len(), 0);
    }

    #[test]
    fn double_artist_double_remixer() {
        let mut track = parse_track("Sonartek & Gorbani - Maqueta (Dubbtone & Tileff Remix)");
        assert_eq!(track.name, "Maqueta");
        assert_eq!(track.artists.len(), 2);
        track.artists.sort();
        assert_eq!(track.artists, ["Gorbani", "Sonartek"]);
        assert_eq!(track.remix.len(), 2);
        track.remix.sort();
        assert_eq!(track.remix, ["Dubbtone", "Tileff"]);
    }

    #[test]
    fn basic() {
        let track = parse_track("Sonartek - Maqueta");
        assert_eq!(track.name, "Maqueta");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists, vec!["Sonartek"]);
        assert_eq!(track.remix.len(), 0);
    }

    #[test]
    fn comma_names() {
        let mut track = parse_track("Gorbani, Maurice Giovannini - Babagam");
        assert_eq!(track.name, "Babagam");
        assert_eq!(track.artists.len(), 2);
        track.artists.sort();
        assert_eq!(track.artists, ["Gorbani", "Maurice Giovannini"]);
        assert_eq!(track.remix.len(), 0);
    }

    #[test]
    fn and_comma_remixer_names() {
        let mut track = parse_track("Dirty Sex Music - Confusion (Bedrud, Grolle & Giese Remix)");
        assert_eq!(track.name, "Confusion");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists, vec!["Dirty Sex Music"]);

        assert_eq!(track.remix.len(), 3);
        track.remix.sort();
        assert_eq!(track.remix, ["Bedrud", "Giese", "Grolle"]);
    }

    // #[test]
    // TODO fix this test
    fn dash_in_track_title() {
        let track = parse_track("Eddie Merced - The last of the Mo-Ricans");
        assert_eq!(track.name, "The last of the Mo-Ricans");
        assert_eq!(track.artists.len(), 1);
        assert_eq!(track.artists, vec!["Eddie Merced"]);
    }
}
