#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    Black,
    White,
}

impl Color {
    pub fn opposite(&self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Piece {
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
                Piece::Pawn => "♟",
                Piece::Knight => "♞",
                Piece::Bishop => "♝",
                Piece::Rook => "♜",
            },
        }
    }
}

#[derive(Clone)]
pub struct BoardState {
    state: [Option<(Color, Piece)>; 64],
    white_can_castle_kingside: bool,
    white_can_castle_queenside: bool,
    black_can_castle_kingside: bool,
    black_can_castle_queenside: bool,
    en_passant: Option<Position>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Position(i8, i8);

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
    const Invalid: Position = Position(-1, -1);

    pub fn from(text: &str) -> Self {
        let mut char_iter = text.chars();

        let (col_ch, row_ch) = match (char_iter.next(), char_iter.next()) {
            (Some(x), Some(y)) => (x, y),
            _ => return Position::Invalid,
        };

        let col = match col_ch {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            'd' => 3,
            'e' => 4,
            'f' => 5,
            'g' => 6,
            'h' => 7,
            c => return Position::Invalid,
        };
        let row = match row_ch {
            '1' => 0,
            '2' => 1,
            '3' => 2,
            '4' => 3,
            '5' => 4,
            '6' => 5,
            '7' => 6,
            '8' => 7,
            c => return Position::Invalid,
        };

        Position(row, col)
    }

    pub fn row(&self) -> i8 {
        self.0
    }

    pub fn col(&self) -> i8 {
        self.1
    }

    pub fn is_valid(&self) -> bool {
        self.0 >= 0 && self.0 <= 7 && self.1 >= 0 && self.1 <= 7
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Move {
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
            white_can_castle_queenside: true,
            white_can_castle_kingside: true,
            black_can_castle_queenside: true,
            black_can_castle_kingside: true,
            en_passant: None,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            state: [None; 64],
            white_can_castle_queenside: true,
            white_can_castle_kingside: true,
            black_can_castle_queenside: true,
            black_can_castle_kingside: true,
            en_passant: None,
        }
    }

    pub fn from_str(state: &str) -> Self {
        let mut line_iter = state.lines().map(|l| l.trim()).peekable();
        // Remove any leading empty lines
        while let Some(l) = line_iter.peek() {
            if l.is_empty() {
                line_iter.next();
            } else {
                break;
            }
        }

        let mut board = Self::new_empty();
        for row in (0..8).rev() {
            // Remove the border line
            line_iter.next().unwrap();
            let pieces = line_iter.next().unwrap();
            let mut chars = pieces.chars();
            // Skip the first character which is an edge piece
            chars.next();
            for (col, ch) in chars.step_by(2).take(8).enumerate() {
                let piece = match ch {
                    '♜' => Some((Color::Black, Piece::Rook)),
                    '♝' => Some((Color::Black, Piece::Bishop)),
                    '♞' => Some((Color::Black, Piece::Knight)),
                    '♟' => Some((Color::Black, Piece::Pawn)),
                    '♛' => Some((Color::Black, Piece::Queen)),
                    '♚' => Some((Color::Black, Piece::King)),
                    '♖' => Some((Color::White, Piece::Rook)),
                    '♗' => Some((Color::White, Piece::Bishop)),
                    '♘' => Some((Color::White, Piece::Knight)),
                    '♙' => Some((Color::White, Piece::Pawn)),
                    '♕' => Some((Color::White, Piece::Queen)),
                    '♔' => Some((Color::White, Piece::King)),
                    _ => None,
                };

                let p = Position(row as i8, col as i8);
                board.set(p, piece);
            }
        }
        board
    }

    pub fn apply_pgn(&mut self, color: Color, pgn: &str) -> Result<(), String> {
        let m = self.pgn_to_move(color, pgn)?;
        self.apply(m);
        Ok(())
    }

    pub fn apply(&mut self, m: Move) {
        self.en_passant = None;

        match m {
            Move::Position(color, piece, original_position, new_position) => {
                if piece == Piece::Pawn {
                    let steps = (new_position.row() - original_position.row());
                    if steps.abs() == 2 {
                        self.en_passant = Some(Position(
                            original_position.row() + steps / 2,
                            new_position.col(),
                        ));
                    }
                }

                if piece == Piece::King {
                    if color == Color::White {
                        self.white_can_castle_kingside = false;
                        self.white_can_castle_queenside = false;
                    } else {
                        self.black_can_castle_kingside = false;
                        self.black_can_castle_queenside = false;
                    }
                }

                if piece == Piece::Rook {
                    if color == Color::White {
                        if original_position == Position(0, 0) || new_position == Position(0, 0) {
                            self.white_can_castle_queenside = false;
                        } else if original_position == Position(0, 7)
                            || new_position == Position(0, 7)
                        {
                            self.white_can_castle_queenside = false;
                        }
                    } else if color == Color::Black {
                        if original_position == Position(7, 0) || new_position == Position(7, 0) {
                            self.black_can_castle_queenside = false;
                        } else if original_position == Position(7, 7)
                            || new_position == Position(7, 7)
                        {
                            self.black_can_castle_queenside = false;
                        }
                    }
                }
                self.set(original_position, None);
                self.set(new_position, Some((color, piece)));
            }
            Move::Takes(color, piece, original_position, new_position) => {
                if piece == Piece::Pawn {
                    // If we are taking an invisible piece, it must be an en passant capture
                    if self.get_position(&new_position).is_none() {
                        let direction = match color {
                            Color::White => -1,
                            Color::Black => 1,
                        };

                        self.set(
                            Position(new_position.row() + direction, new_position.col()),
                            None,
                        );
                    }
                }

                self.set(original_position, None);
                self.set(new_position, Some((color, piece)));
            }
            Move::Promotion(color, original_position, new_position, piece) => {
                self.set(original_position, None);
                self.set(new_position, Some((color, piece)));
            }
            Move::CastleKingside(color) => {
                if color == Color::White {
                    self.set(Position::from("g1"), Some((Color::White, Piece::King)));
                    self.set(Position::from("e1"), None);
                    self.set(Position::from("f1"), Some((Color::White, Piece::Rook)));
                    self.set(Position::from("h1"), None);
                    self.white_can_castle_kingside = false;
                    self.white_can_castle_queenside = false;
                } else {
                    self.set(Position::from("g8"), Some((Color::Black, Piece::King)));
                    self.set(Position::from("e8"), None);
                    self.set(Position::from("f8"), Some((Color::Black, Piece::Rook)));
                    self.set(Position::from("h8"), None);
                    self.black_can_castle_kingside = false;
                    self.black_can_castle_queenside = false;
                }
            }
            Move::CastleQueenside(color) => {
                if color == Color::White {
                    self.set(Position::from("c1"), Some((Color::White, Piece::King)));
                    self.set(Position::from("e1"), None);
                    self.set(Position::from("d1"), Some((Color::White, Piece::Rook)));
                    self.set(Position::from("a1"), None);
                    self.white_can_castle_kingside = false;
                    self.white_can_castle_queenside = false;
                } else {
                    self.set(Position::from("c8"), Some((Color::Black, Piece::King)));
                    self.set(Position::from("e8"), None);
                    self.set(Position::from("d8"), Some((Color::Black, Piece::Rook)));
                    self.set(Position::from("a8"), None);
                    self.black_can_castle_kingside = false;
                    self.black_can_castle_queenside = false;
                }
            }
        }
    }

    pub fn pgn_to_move(&self, color: Color, code: &str) -> Result<Move, String> {
        let code = if code.ends_with("#") || code.ends_with("+") {
            &code[..code.len() - 1]
        } else {
            code
        };

        if code == "O-O" {
            return Ok(Move::CastleKingside(color));
        } else if code == "O-O-O" {
            return Ok(Move::CastleQueenside(color));
        }

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

        for m in self.get_legal_moves(&color) {
            let (p, start, end) = match m {
                Move::Position(_, p, start, end) => (p, start, end),
                Move::Takes(_, p, start, end) => (p, start, end),
                Move::Promotion(_, start, end, _) => (Piece::Pawn, start, end),
                _ => continue,
            };

            if p != piece {
                continue;
            }
            if end != Position::from(position) {
                continue;
            }
            if let Some(hint) = position_hint {
                if hint.len() == 2 {
                    if start != Position::from(hint) {
                        continue;
                    }
                }
                if let Some(h) = hint.chars().next() {
                    if h.is_alphabetic() && Position::col_to_char(start.col()) != h {
                        continue;
                    } else if h.is_numeric() && Position::row_to_char(start.row()) != h {
                        continue;
                    }
                }
            }

            if let Move::Promotion(_, _, _, promoted) = m {
                if let Some(promoted_to) = promotes {
                    if promoted != promoted_to {
                        continue;
                    }
                }
            }

            return Ok(m);
        }

        Err(format!("{} is not a legal move!", code))
    }

    // Determines if the current board state puts a player in check
    pub fn is_in_check(&self, color: &Color) -> bool {
        let king_pos = match self.get_king_position(color) {
            Some(p) => p,
            None => return false,
        };

        for row in 0..8 {
            for col in 0..8 {
                if let Some((c, p)) = self.get(row, col) {
                    if c == color {
                        continue;
                    }

                    match p {
                        Piece::Pawn => {
                            let direction: i8 = match c {
                                Color::White => 1,
                                Color::Black => -1,
                            };

                            if row + direction == king_pos.row()
                                && (king_pos.col() - col).abs() == 1
                            {
                                return true;
                            }
                        }
                        Piece::Knight => {
                            let dx = (row - king_pos.row()).abs();
                            let dy = (col - king_pos.col()).abs();
                            if dx == 2 && dy == 1 || dx == 1 && dy == 2 {
                                return true;
                            }
                        }
                        Piece::Rook => {
                            if self.is_line_of_sight(
                                Position(row, col),
                                king_pos,
                                &[(0, 1), (0, -1), (1, 0), (-1, 0)],
                            ) {
                                return true;
                            }
                        }
                        Piece::Queen => {
                            if self.is_line_of_sight(
                                Position(row, col),
                                king_pos,
                                &[
                                    (0, 1),
                                    (0, -1),
                                    (1, 0),
                                    (-1, 0),
                                    (1, 1),
                                    (1, -1),
                                    (-1, -1),
                                    (-1, 1),
                                ],
                            ) {
                                return true;
                            }
                        }
                        Piece::Bishop => {
                            if self.is_line_of_sight(
                                Position(row, col),
                                king_pos,
                                &[(1, 1), (1, -1), (-1, -1), (-1, 1)],
                            ) {
                                return true;
                            }
                        }
                        Piece::King => {
                            let dx = (row - king_pos.row()).abs();
                            let dy = (col - king_pos.col()).abs();
                            if dx <= 1 && dy <= 1 {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    pub fn is_line_of_sight(
        &self,
        start: Position,
        end: Position,
        directions: &[(i8, i8)],
    ) -> bool {
        let drow = end.row() - start.row();
        let dcol = end.col() - start.col();

        for (dr, dc) in directions {
            if drow * dc == dcol * dr && drow * dr >= 0 && dcol * dc >= 0 {
                // A line of sight may exist, so check all squares for blockers
                let mut in_check = true;
                for i in 1..7 {
                    let p = Position(start.row() + i * dr, start.col() + i * dc);
                    if !p.is_valid() {
                        break;
                    }
                    if p == end {
                        break;
                    }
                    if let Some(_) = self.get_position(&p) {
                        in_check = false;
                        break;
                    }
                }
                if in_check {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_king_position(&self, color: &Color) -> Option<Position> {
        for row in 0..8 {
            for col in 0..8 {
                if let Some((c, Piece::King)) = self.get(row, col) {
                    if c == color {
                        return Some(Position(row, col));
                    }
                }
            }
        }
        None
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
                                // If we reach the end of the board with a pawn, promote
                                if row + direction == 0 || row + direction == 7 {
                                    moves.push(Move::Promotion(
                                        *color,
                                        Position(row, col),
                                        Position(row + direction, col),
                                        Piece::Queen,
                                    ));
                                    moves.push(Move::Promotion(
                                        *color,
                                        Position(row, col),
                                        Position(row + direction, col),
                                        Piece::Knight,
                                    ));
                                    moves.push(Move::Promotion(
                                        *color,
                                        Position(row, col),
                                        Position(row + direction, col),
                                        Piece::Bishop,
                                    ));
                                    moves.push(Move::Promotion(
                                        *color,
                                        Position(row, col),
                                        Position(row + direction, col),
                                        Piece::Rook,
                                    ));
                                } else {
                                    // Check if this is the first move for the pawn, in which case
                                    // we can move twice.
                                    if row == 1 && *color == Color::White
                                        || row == 7 && *color == Color::Black
                                    {
                                        if self.get(row + direction * 2, col).is_none() {
                                            moves.push(Move::Position(
                                                *color,
                                                Piece::Pawn,
                                                Position(row, col),
                                                Position(row + direction * 2, col),
                                            ));
                                        }
                                    }
                                    moves.push(Move::Position(
                                        *color,
                                        Piece::Pawn,
                                        Position(row, col),
                                        Position(row + direction, col),
                                    ));
                                }
                            }

                            if let Some(pos) = self.en_passant {
                                if pos == Position(row + direction, col - 1) {
                                    moves.push(Move::Takes(
                                        *color,
                                        Piece::Pawn,
                                        Position(row, col),
                                        Position(row + direction, col - 1),
                                    ))
                                } else if pos == Position(row + direction, col + 1) {
                                    moves.push(Move::Takes(
                                        *color,
                                        Piece::Pawn,
                                        Position(row, col),
                                        Position(row + direction, col + 1),
                                    ))
                                }
                            }

                            if let Some((c, p)) = self.get(row + direction, col - 1) {
                                if color != c {
                                    if row + direction == 0 || row + direction == 7 {
                                        moves.push(Move::Promotion(
                                            *color,
                                            Position(row, col),
                                            Position(row + direction, col - 1),
                                            Piece::Queen,
                                        ));
                                        moves.push(Move::Promotion(
                                            *color,
                                            Position(row, col),
                                            Position(row + direction, col - 1),
                                            Piece::Knight,
                                        ));
                                        moves.push(Move::Promotion(
                                            *color,
                                            Position(row, col),
                                            Position(row + direction, col - 1),
                                            Piece::Bishop,
                                        ));
                                        moves.push(Move::Promotion(
                                            *color,
                                            Position(row, col),
                                            Position(row + direction, col - 1),
                                            Piece::Rook,
                                        ));
                                    } else {
                                        moves.push(Move::Takes(
                                            *color,
                                            Piece::Pawn,
                                            Position(row, col),
                                            Position(row + direction, col - 1),
                                        ))
                                    }
                                }
                            }
                            if let Some((c, p)) = self.get(row + direction, col + 1) {
                                if color != c {
                                    if row + direction == 0 || row + direction == 7 {
                                        moves.push(Move::Promotion(
                                            *color,
                                            Position(row, col),
                                            Position(row + direction, col + 1),
                                            Piece::Queen,
                                        ));
                                        moves.push(Move::Promotion(
                                            *color,
                                            Position(row, col),
                                            Position(row + direction, col + 1),
                                            Piece::Knight,
                                        ));
                                        moves.push(Move::Promotion(
                                            *color,
                                            Position(row, col),
                                            Position(row + direction, col + 1),
                                            Piece::Bishop,
                                        ));
                                        moves.push(Move::Promotion(
                                            *color,
                                            Position(row, col),
                                            Position(row + direction, col + 1),
                                            Piece::Rook,
                                        ));
                                    } else {
                                        moves.push(Move::Takes(
                                            *color,
                                            Piece::Pawn,
                                            Position(row, col),
                                            Position(row + direction, col + 1),
                                        ))
                                    }
                                }
                            }
                        }
                        Piece::Knight => {
                            let possible_moves = &[
                                Position(row - 1, col - 2),
                                Position(row + 1, col - 2),
                                Position(row + 1, col + 2),
                                Position(row - 1, col + 2),
                                Position(row - 2, col - 1),
                                Position(row + 2, col - 1),
                                Position(row + 2, col + 1),
                                Position(row - 2, col + 1),
                            ];

                            for position in possible_moves {
                                if !position.is_valid() {
                                    continue;
                                }

                                if let Some((c, p)) = self.get_position(position) {
                                    if color != c {
                                        moves.push(Move::Takes(
                                            *color,
                                            Piece::Knight,
                                            Position(row, col),
                                            *position,
                                        ));
                                    }
                                } else {
                                    moves.push(Move::Position(
                                        *color,
                                        Piece::Knight,
                                        Position(row, col),
                                        *position,
                                    ))
                                }
                            }
                        }
                        Piece::Rook => {
                            let directions = &[(0, 1), (0, -1), (1, 0), (-1, 0)];
                            self.get_legal_moves_along_directions(
                                *color,
                                row,
                                col,
                                directions,
                                Piece::Rook,
                                &mut moves,
                            );
                        }
                        Piece::Bishop => {
                            let directions = &[(1, 1), (1, -1), (-1, 1), (-1, -1)];
                            self.get_legal_moves_along_directions(
                                *color,
                                row,
                                col,
                                directions,
                                Piece::Bishop,
                                &mut moves,
                            );
                        }
                        Piece::Queen => {
                            let directions = &[
                                (1, 1),
                                (1, -1),
                                (-1, 1),
                                (-1, -1),
                                (0, 1),
                                (0, -1),
                                (1, 0),
                                (-1, 0),
                            ];
                            self.get_legal_moves_along_directions(
                                *color,
                                row,
                                col,
                                directions,
                                Piece::Queen,
                                &mut moves,
                            );
                        }
                        Piece::King => {
                            let directions = &[
                                (1, 1),
                                (1, -1),
                                (-1, 1),
                                (-1, -1),
                                (0, 1),
                                (0, -1),
                                (1, 0),
                                (-1, 0),
                            ];
                            for (drow, dcol) in directions {
                                let p = Position(row + drow, col + dcol);
                                if !p.is_valid() {
                                    continue;
                                }

                                if let Some((c, _)) = self.get_position(&p) {
                                    if color != c {
                                        moves.push(Move::Takes(
                                            *color,
                                            Piece::King,
                                            Position(row, col),
                                            p,
                                        ))
                                    }
                                } else {
                                    moves.push(Move::Position(
                                        *color,
                                        Piece::King,
                                        Position(row, col),
                                        p,
                                    ));
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }

        let is_in_check = self.is_in_check(color);

        // Check whether castling is possible
        if !is_in_check {
            if color == &Color::White {
                if self.white_can_castle_queenside {
                    if let Some((Color::White, Piece::Rook)) = self.get(0, 0) {
                        if let Some((Color::White, Piece::King)) = self.get(0, 4) {
                            if self.get(0, 1).is_none()
                                && self.get(0, 2).is_none()
                                && self.get(0, 3).is_none()
                            {
                                // We also need to ensure that the king does not move into check
                                // while castling.
                                let mut b = self.clone();
                                b.apply(Move::Position(
                                    *color,
                                    Piece::King,
                                    Position(0, 4),
                                    Position(0, 3),
                                ));
                                if !b.is_in_check(color) {
                                    moves.push(Move::CastleQueenside(*color));
                                }
                            }
                        }
                    }
                }
                if self.white_can_castle_kingside {
                    if let Some((Color::White, Piece::Rook)) = self.get(0, 7) {
                        if let Some((Color::White, Piece::King)) = self.get(0, 4) {
                            if self.get(0, 6).is_none() && self.get(0, 5).is_none() {
                                // We also need to ensure that the king does not move into check
                                // while castling.
                                let mut b = self.clone();
                                b.apply(Move::Position(
                                    *color,
                                    Piece::King,
                                    Position(0, 4),
                                    Position(0, 5),
                                ));
                                if !b.is_in_check(color) {
                                    moves.push(Move::CastleKingside(*color));
                                }
                            }
                        }
                    }
                }
            } else {
                if self.black_can_castle_queenside {
                    if let Some((Color::Black, Piece::Rook)) = self.get(7, 0) {
                        if let Some((Color::Black, Piece::King)) = self.get(7, 4) {
                            if self.get(7, 1).is_none()
                                && self.get(7, 2).is_none()
                                && self.get(7, 3).is_none()
                            {
                                // We also need to ensure that the king does not move into check
                                // while castling.
                                let mut b = self.clone();
                                b.apply(Move::Position(
                                    *color,
                                    Piece::King,
                                    Position(7, 4),
                                    Position(7, 3),
                                ));
                                if !b.is_in_check(color) {
                                    moves.push(Move::CastleQueenside(*color));
                                }
                            }
                        }
                    }
                }
                if self.black_can_castle_kingside {
                    if let Some((Color::Black, Piece::Rook)) = self.get(7, 7) {
                        if let Some((Color::Black, Piece::King)) = self.get(7, 4) {
                            if self.get(7, 6).is_none() && self.get(7, 5).is_none() {
                                // We also need to ensure that the king does not move into check
                                // while castling.
                                let mut b = self.clone();
                                b.apply(Move::Position(
                                    *color,
                                    Piece::King,
                                    Position(7, 4),
                                    Position(7, 5),
                                ));
                                if !b.is_in_check(color) {
                                    moves.push(Move::CastleKingside(*color));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Exclude any moves that would lead to check
        moves
            .into_iter()
            .filter(|m| {
                let mut b = self.clone();
                b.apply(*m);
                !b.is_in_check(color)
            })
            .collect()
    }

    pub fn get_legal_moves_along_directions(
        &self,
        color: Color,
        row: i8,
        col: i8,
        directions: &[(i8, i8)],
        piece: Piece,
        moves: &mut Vec<Move>,
    ) {
        for (drow, dcol) in directions {
            for i in (1..8) {
                let p = Position(row + i * drow, col + i * dcol);
                if !p.is_valid() {
                    break;
                }

                if let Some((c, _)) = self.get_position(&p) {
                    if color != *c {
                        moves.push(Move::Takes(color, piece, Position(row, col), p))
                    }
                    break;
                }
                moves.push(Move::Position(color, piece, Position(row, col), p));
            }
        }
    }

    pub fn get_position(&self, position: &Position) -> Option<&(Color, Piece)> {
        self.get(position.0, position.1)
    }

    pub fn get(&self, row: i8, col: i8) -> Option<&(Color, Piece)> {
        if row >= 0 && row <= 7 && col >= 0 && col <= 7 {
            self.state[(row * 8 + col) as usize].as_ref()
        } else {
            None
        }
    }

    pub fn set(&mut self, p: Position, state: Option<(Color, Piece)>) {
        self.state[(p.row() * 8 + p.col()) as usize] = state;
    }

    pub fn render(&self, with_guides: bool) -> String {
        let mut output = String::new();

        if with_guides {
            output += "   a b c d e f g h\n";
        }

        for row in (0..8).rev() {
            if with_guides {
                output += "  ";
            }

            if row == 7 {
                output += "┌─┬─┬─┬─┬─┬─┬─┬─┐\n";
            } else {
                output += "├─┼─┼─┼─┼─┼─┼─┼─┤\n";
            }

            if with_guides {
                output.push(Position::row_to_char(row));
                output.push(' ');
            }
            output.push('│');

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

        if with_guides {
            output += "  ";
        }

        output += "└─┴─┴─┴─┴─┴─┴─┴─┘";

        if with_guides {
            output += "\n   a b c d e f g h\n";
        }
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
        │♜│♞│♝│♛│♚│♝│♞│♜│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♟│♟│♟│♟│♟│♟│♟│♟│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♙│♙│♙│♙│♙│♙│♙│♙│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♖│♘│♗│♕│♔│♗│♘│♖│\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘";

        println!("{}", b.render(false));
        assert_eq!(&b.render(false), expected);
    }

    #[test]
    fn test_from_str() {
        let rendered_board = "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        │♜│♞│♝│♛│ │♝│♞│♜│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♟│♟│♟│♟│♚│♟│♟│♟│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │♟│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │♙│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │♘│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♙│♙│♙│♙│ │♙│♙│♙│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♖│♘│♗│♕│♔│♗│ │♖│\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘";
        let b = BoardState::from_str(rendered_board);

        println!("{}", b.render(false));
        assert_eq!(&b.render(false), rendered_board);
    }

    #[test]
    fn test_pawn_capture() {
        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │♟│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │♙│♙│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::White);
        assert_eq!(
            moves,
            vec![
                Move::Position(
                    Color::White,
                    Piece::Pawn,
                    Position::from("d4"),
                    Position::from("d5")
                ),
                Move::Takes(
                    Color::White,
                    Piece::Pawn,
                    Position::from("d4"),
                    Position::from("e5")
                ),
            ]
        );

        let moves = b.get_legal_moves(&Color::Black);
        assert_eq!(
            moves,
            vec![Move::Takes(
                Color::Black,
                Piece::Pawn,
                Position::from("e5"),
                Position::from("d4")
            ),]
        );

        let expected = "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │♟│♙│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘";

        b.apply(moves[0]);

        assert_eq!(b.render(false), expected);
    }

    #[test]
    fn test_knight_moves() {
        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │♞│ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::Black);
        assert_eq!(
            moves,
            vec![
                Move::Position(
                    Color::Black,
                    Piece::Knight,
                    Position::from("g6"),
                    Position::from("e5")
                ),
                Move::Position(
                    Color::Black,
                    Piece::Knight,
                    Position::from("g6"),
                    Position::from("e7")
                ),
                Move::Takes(
                    Color::Black,
                    Piece::Knight,
                    Position::from("g6"),
                    Position::from("f4")
                ),
                Move::Position(
                    Color::Black,
                    Piece::Knight,
                    Position::from("g6"),
                    Position::from("f8")
                ),
                Move::Position(
                    Color::Black,
                    Piece::Knight,
                    Position::from("g6"),
                    Position::from("h8")
                ),
                Move::Position(
                    Color::Black,
                    Piece::Knight,
                    Position::from("g6"),
                    Position::from("h4")
                ),
            ]
        );
    }

    #[test]
    fn test_rook_moves() {
        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♞│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │♖│ │ │♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::White);
        assert_eq!(
            moves,
            vec![
                Move::Position(
                    Color::White,
                    Piece::Rook,
                    Position::from("c4"),
                    Position::from("d4")
                ),
                Move::Position(
                    Color::White,
                    Piece::Rook,
                    Position::from("c4"),
                    Position::from("e4")
                ),
                Move::Position(
                    Color::White,
                    Piece::Rook,
                    Position::from("c4"),
                    Position::from("b4")
                ),
                Move::Position(
                    Color::White,
                    Piece::Rook,
                    Position::from("c4"),
                    Position::from("a4")
                ),
                Move::Position(
                    Color::White,
                    Piece::Rook,
                    Position::from("c4"),
                    Position::from("c5")
                ),
                Move::Takes(
                    Color::White,
                    Piece::Rook,
                    Position::from("c4"),
                    Position::from("c6")
                ),
                Move::Position(
                    Color::White,
                    Piece::Rook,
                    Position::from("c4"),
                    Position::from("c3")
                ),
                Move::Takes(
                    Color::White,
                    Piece::Rook,
                    Position::from("c4"),
                    Position::from("c2")
                ),
            ]
        );
    }

    #[test]
    fn test_bishop_moves() {
        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │♗│ │ │ │♞│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♞│ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::White);
        assert_eq!(
            moves,
            vec![
                Move::Takes(
                    Color::White,
                    Piece::Bishop,
                    Position::from("b5"),
                    Position::from("c6")
                ),
                Move::Position(
                    Color::White,
                    Piece::Bishop,
                    Position::from("b5"),
                    Position::from("a6")
                ),
                Move::Position(
                    Color::White,
                    Piece::Bishop,
                    Position::from("b5"),
                    Position::from("c4")
                ),
                Move::Takes(
                    Color::White,
                    Piece::Bishop,
                    Position::from("b5"),
                    Position::from("d3")
                ),
                Move::Position(
                    Color::White,
                    Piece::Bishop,
                    Position::from("b5"),
                    Position::from("a4")
                ),
            ]
        );
    }

    #[test]
    fn test_king_moves() {
        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♞│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │♔│♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♞│ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::White);
        assert_eq!(
            moves,
            vec![
                Move::Takes(
                    Color::White,
                    Piece::King,
                    Position::from("e4"),
                    Position::from("f5")
                ),
                Move::Position(
                    Color::White,
                    Piece::King,
                    Position::from("e4"),
                    Position::from("d5")
                ),
                Move::Position(
                    Color::White,
                    Piece::King,
                    Position::from("e4"),
                    Position::from("f3")
                ),
                Move::Takes(
                    Color::White,
                    Piece::King,
                    Position::from("e4"),
                    Position::from("d3")
                ),
                // King can't move to e4, e3 or e5 due to check
            ]
        );
    }

    #[test]
    fn test_is_check() {
        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♞│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │♔│♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♞│ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        assert_eq!(b.is_in_check(&Color::White), false);

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♞│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♞│♔│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        assert_eq!(b.is_in_check(&Color::White), true);

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♟│ │♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♞│♔│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        assert_eq!(b.is_in_check(&Color::White), true);

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │♛│♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♙│ │♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♞│♔│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        assert_eq!(b.is_in_check(&Color::White), false);

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │♛│♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♞│♔│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        assert_eq!(b.is_in_check(&Color::White), true);

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♙│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │♞│♔│ │ │♜│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        assert_eq!(b.is_in_check(&Color::White), true);
    }

    fn assert_contains(parents: &[Move], children: &[Move]) {
        for child in children {
            let mut seen = false;
            for parent in parents {
                if parent == child {
                    seen = true;
                    break;
                }
            }
            assert!(seen, "couldn't find {:?} in {:?}!", child, parents);
        }
    }

    fn assert_excludes(parents: &[Move], children: &[Move]) {
        for child in children {
            for parent in parents {
                if parent == child {
                    assert!(
                        false,
                        "expected not to find {:?}!\n\nin: {:?}!",
                        child, parents
                    );
                }
            }
        }
    }

    #[test]
    fn test_castling() {
        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        |♖│ │ │ │♔│ │ │♖│\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::White);
        assert_contains(
            &moves,
            &[
                Move::CastleKingside(Color::White),
                Move::CastleQueenside(Color::White),
            ],
        );

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        |♖│♞│ │ │♔│ │ │♖│\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::White);
        assert_contains(&moves, &[Move::CastleKingside(Color::White)]);
        assert_excludes(&moves, &[Move::CastleQueenside(Color::White)]);

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♖│♞│ │ │♔│ │ │♖│\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::White);
        assert_excludes(
            &moves,
            &[
                Move::CastleKingside(Color::White),
                Move::CastleQueenside(Color::White),
            ],
        );

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │♜│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │♖│ │ │♔│ │ │♖│\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::White);
        assert_excludes(
            &moves,
            &[
                Move::CastleKingside(Color::White),
                Move::CastleQueenside(Color::White),
            ],
        );

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        |♖│ │ │ │♔│ │ │♖│\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        b.apply(Move::Position(
            Color::White,
            Piece::King,
            Position::from("e1"),
            Position::from("e2"),
        ));
        b.apply(Move::Position(
            Color::White,
            Piece::King,
            Position::from("e2"),
            Position::from("e1"),
        ));

        let moves = b.get_legal_moves(&Color::White);
        assert_excludes(
            &moves,
            &[
                Move::CastleKingside(Color::White),
                Move::CastleQueenside(Color::White),
            ],
        );

        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        |♜│ │ │ │♚│ │ │♜│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        | │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let moves = b.get_legal_moves(&Color::Black);
        assert_contains(
            &moves,
            &[
                Move::CastleKingside(Color::Black),
                Move::CastleQueenside(Color::Black),
            ],
        );

        let expected = "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        │♜│ │ │ │ │♜│♚│ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘";

        b.apply(Move::CastleKingside(Color::Black));
        assert_eq!(&b.render(false), expected);
    }

    #[test]
    fn test_en_passant() {
        let mut b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │♟│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │♙│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );
        b.apply_pgn(Color::White, "e4").unwrap();

        let moves = b.get_legal_moves(&Color::Black);
        assert_eq!(
            &moves,
            &[
                Move::Position(
                    Color::Black,
                    Piece::Pawn,
                    Position::from("f4"),
                    Position::from("f3"),
                ),
                Move::Takes(
                    Color::Black,
                    Piece::Pawn,
                    Position::from("f4"),
                    Position::from("e3"),
                )
            ],
        );

        b.apply(moves[1]);

        let expected = "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │♟│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘";

        println!("{}", b.render(false));

        assert_eq!(&b.render(false), expected);
    }

    #[test]
    fn test_pgn() {
        let b = BoardState::new();
        assert_eq!(
            b.pgn_to_move(Color::Black, "O-O"),
            Ok(Move::CastleKingside(Color::Black))
        );

        assert_eq!(
            b.pgn_to_move(Color::White, "e3"),
            Ok(Move::Position(
                Color::White,
                Piece::Pawn,
                Position::from("e2"),
                Position::from("e3"),
            ))
        );

        assert_eq!(
            b.pgn_to_move(Color::White, "Nc3"),
            Ok(Move::Position(
                Color::White,
                Piece::Knight,
                Position::from("b1"),
                Position::from("c3"),
            ))
        );

        assert!(b.pgn_to_move(Color::White, "").is_err());
        assert!(b.pgn_to_move(Color::White, "Yc5").is_err());
        assert!(b.pgn_to_move(Color::White, "Rc3").is_err());
        assert!(b.pgn_to_move(Color::White, "N83").is_err());
    }
}
