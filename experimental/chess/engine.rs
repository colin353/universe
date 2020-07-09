#[derive(Debug, Copy, Clone, PartialEq)]
enum Color {
    Black,
    White,
}

#[derive(Debug, PartialEq)]
enum Piece {
    King,
    Queen,
    Knight,
    Pawn,
    Bishop,
    Rook,
}

impl Piece {
    pub fn to_str(&self, color: &Color) -> &'static str {
        match color {
            Color::White => match self {
                Piece::King => "♔",
                Piece::Queen => "♕",
                Piece::Pawn => "♙",
                Piece::Knight => "♘",
                Piece::Bishop => "♗",
                Piece::Rook => "♖",
            },
            Color::Black => match self {
                Piece::King => "♚",
                Piece::Queen => "♛",
                Piece::Pawn => "♟︎",
                Piece::Knight => "♞",
                Piece::Bishop => "♝",
                Piece::Rook => "♜",
            },
        }
    }
}

struct BoardState {
    state: [Option<(Color, Piece)>; 64],
}

#[derive(Clone, PartialEq)]
struct Position(i8, i8);

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            Position::col_to_char(self.col()),
            Position::row_to_char(self.row()),
        )
    }
}
impl std::fmt::Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            Position::col_to_char(self.col()),
            Position::row_to_char(self.row()),
        )
    }
}

impl Position {
    pub fn from(text: &str) -> Self {
        let mut char_iter = text.chars();
        let col = match char_iter.next().unwrap() {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            'd' => 3,
            'e' => 4,
            'f' => 5,
            'g' => 6,
            'h' => 7,
            c => panic!("unknown col `{}`!", c),
        };
        let row = match char_iter.next().unwrap() {
            '1' => 0,
            '2' => 1,
            '3' => 2,
            '4' => 3,
            '5' => 4,
            '6' => 5,
            '7' => 6,
            '8' => 7,
            c => panic!("unknown row `{}`!", c),
        };

        Position(row, col)
    }

    pub fn row(&self) -> i8 {
        self.0
    }

    pub fn col(&self) -> i8 {
        self.1
    }

    pub fn col_to_char(col: i8) -> char {
        match col {
            0 => 'a',
            1 => 'b',
            2 => 'c',
            3 => 'd',
            4 => 'e',
            5 => 'f',
            6 => 'g',
            7 => 'h',
            c => panic!("unknown col `{}`!", c),
        }
    }

    pub fn row_to_char(row: i8) -> char {
        match row {
            0 => '1',
            1 => '2',
            2 => '3',
            3 => '4',
            4 => '5',
            5 => '6',
            6 => '7',
            7 => '8',
            r => panic!("unknown row `{}`!", r),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Move {
    Position(Color, Piece, Position, Position),
    Promotion(Color, Position, Position, Piece),
    Takes(Color, Piece, Position, Position),
    CastleKingside(Color),
    CastleQueenside(Color),
}

impl BoardState {
    pub fn new() -> Self {
        Self {
            state: [
                Some((Color::White, Piece::Rook)),
                Some((Color::White, Piece::Knight)),
                Some((Color::White, Piece::Bishop)),
                Some((Color::White, Piece::Queen)),
                Some((Color::White, Piece::King)),
                Some((Color::White, Piece::Bishop)),
                Some((Color::White, Piece::Knight)),
                Some((Color::White, Piece::Rook)),
                Some((Color::White, Piece::Pawn)),
                Some((Color::White, Piece::Pawn)),
                Some((Color::White, Piece::Pawn)),
                Some((Color::White, Piece::Pawn)),
                Some((Color::White, Piece::Pawn)),
                Some((Color::White, Piece::Pawn)),
                Some((Color::White, Piece::Pawn)),
                Some((Color::White, Piece::Pawn)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some((Color::Black, Piece::Pawn)),
                Some((Color::Black, Piece::Pawn)),
                Some((Color::Black, Piece::Pawn)),
                Some((Color::Black, Piece::Pawn)),
                Some((Color::Black, Piece::Pawn)),
                Some((Color::Black, Piece::Pawn)),
                Some((Color::Black, Piece::Pawn)),
                Some((Color::Black, Piece::Pawn)),
                Some((Color::Black, Piece::Rook)),
                Some((Color::Black, Piece::Knight)),
                Some((Color::Black, Piece::Bishop)),
                Some((Color::Black, Piece::Queen)),
                Some((Color::Black, Piece::King)),
                Some((Color::Black, Piece::Bishop)),
                Some((Color::Black, Piece::Knight)),
                Some((Color::Black, Piece::Rook)),
            ],
        }
    }

    pub fn apply(&mut self, m: Move) {}

    pub fn pgn_to_move(&self, color: Color, code: &str) -> Move {
        if code == "O-O" {
            return Move::CastleKingside(color);
        } else if code == "O-O-O" {
            return Move::CastleQueenside(color);
        }

        let code = if code.ends_with("#") || code.ends_with("+") {
            &code[..code.len() - 1]
        } else {
            code
        };

        let mut char_iter = code.chars().peekable();

        let piece = match char_iter.peek() {
            Some('N') => Piece::Knight,
            Some('B') => Piece::Bishop,
            Some('R') => Piece::Rook,
            Some('Q') => Piece::Queen,
            Some('K') => Piece::King,
            _ => Piece::Pawn,
        };

        if piece != Piece::Pawn {
            char_iter.next();
        }

        let mut position_spec = String::new();
        let mut promotes = None;
        let mut takes = false;
        while let Some(x) = char_iter.peek() {
            if x == &'x' {
                takes = true;
                char_iter.next();
            } else if x == &'=' {
                char_iter.next();
                promotes = match char_iter.next() {
                    Some('N') => Some(Piece::Knight),
                    Some('B') => Some(Piece::Bishop),
                    Some('R') => Some(Piece::Rook),
                    Some('Q') => Some(Piece::Queen),
                    Some('K') => Some(Piece::King),
                    _ => None,
                };
                break;
            } else {
                position_spec.push(char_iter.next().unwrap());
            }
        }

        let (position_hint, position) = if position_spec.len() > 2 {
            (
                Some(&position_spec[..position_spec.len() - 2]),
                &position_spec[position_spec.len() - 2..],
            )
        } else {
            (None, position_spec.as_str())
        };

        let destination = Position::from(position);
        if let Some(piece) = promotes {
            return Move::Promotion(color, destination.clone(), destination, piece);
        }

        if takes {
            return Move::Takes(color, piece, destination.clone(), destination);
        }

        Move::Position(color, piece, destination.clone(), destination)
    }

    pub fn get_legal_moves(&self, color: &Color) -> Vec<Move> {
        let direction: i8 = match color {
            Color::White => 1,
            Color::Black => -1,
        };

        let mut moves = Vec::new();

        for row in 0..8 {
            for col in 0..8 {
                if let Some((c, p)) = self.get(row, col) {
                    if c != color {
                        continue;
                    }

                    match p {
                        Piece::Pawn => {
                            if self.get(row + direction, col).is_none() {
                                moves.push(Move::Position(
                                    *color,
                                    Piece::Pawn,
                                    Position(row, col),
                                    Position(row + direction, col),
                                ))
                            }
                        }
                        _ => (),
                    }
                }
            }
        }

        moves
    }

    pub fn get_position(&self, position: &Position) -> Option<&(Color, Piece)> {
        self.get(position.0, position.1)
    }

    pub fn get(&self, row: i8, col: i8) -> Option<&(Color, Piece)> {
        self.state[(row * 8 + col) as usize].as_ref()
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        for row in (0..8).rev() {
            if row == 7 {
                output += "┌─┬─┬─┬─┬─┬─┬─┬─┐\n|";
            } else {
                output += "├─┼─┼─┼─┼─┼─┼─┼─┤\n|";
            }

            for col in 0..8 {
                match self.get(row, col) {
                    Some((c, p)) => {
                        output += p.to_str(c);
                        output += "│";
                    }
                    None => output += " │",
                }
            }
            output += "\n";
        }
        output += "└─┴─┴─┴─┴─┴─┴─┴─┘";
        output
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_render() {
        let b = BoardState::new();
        let expected = "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        |♜│♞│♝│♛│♚│♝│♞│♜│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        |♟︎│♟︎│♟︎│♟︎│♟︎│♟︎│♟︎│♟︎│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        |♙│♙│♙│♙│♙│♙│♙│♙│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        |♖│♘│♗│♕│♔│♗│♘│♖│\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘";

        println!("{}", b.render());
        assert_eq!(&b.render(), expected);
    }

    #[test]
    fn test_pgn() {
        let b = BoardState::new();
        assert_eq!(
            b.pgn_to_move(Color::Black, "O-O"),
            Move::CastleKingside(Color::Black)
        );
    }

    #[test]
    fn test_legal_moves() {
        let b = BoardState::new();
        assert_eq!(
            b.get_legal_moves(&Color::White),
            vec![
                Move::Position(
                    Color::White,
                    Piece::Pawn,
                    Position::from("a2"),
                    Position::from("a3")
                ),
                Move::Position(
                    Color::White,
                    Piece::Pawn,
                    Position::from("b2"),
                    Position::from("b3")
                ),
                Move::Position(
                    Color::White,
                    Piece::Pawn,
                    Position::from("c2"),
                    Position::from("c3")
                ),
                Move::Position(
                    Color::White,
                    Piece::Pawn,
                    Position::from("d2"),
                    Position::from("d3")
                ),
                Move::Position(
                    Color::White,
                    Piece::Pawn,
                    Position::from("e2"),
                    Position::from("e3")
                ),
                Move::Position(
                    Color::White,
                    Piece::Pawn,
                    Position::from("f2"),
                    Position::from("f3")
                ),
                Move::Position(
                    Color::White,
                    Piece::Pawn,
                    Position::from("g2"),
                    Position::from("g3")
                ),
                Move::Position(
                    Color::White,
                    Piece::Pawn,
                    Position::from("h2"),
                    Position::from("h3")
                ),
            ]
        );
    }
}
