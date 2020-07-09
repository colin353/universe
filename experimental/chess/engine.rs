#[derive(Debug, Copy, Clone)]
enum Color {
    Black,
    White,
}

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

type Position = (u8, u8);

enum Move {
    Position(Color, Piece, Position, Position),
    Promotion(Color, Position, Position, Piece),
    Takes(Color, Piece, Position, Position),
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

    pub fn get(&self, row: u8, col: u8) -> Option<&(Color, Piece)> {
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

            for col in (0..8) {
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
    fn test_map() {
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
}
