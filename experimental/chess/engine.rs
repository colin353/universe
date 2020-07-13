pub use chess::*;

const DEPTH_LIMIT: u8 = 3;

pub struct Evaluator {
    depth_limit: u8,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            depth_limit: DEPTH_LIMIT,
        }
    }

    pub fn evaluate(&self, board: &BoardState, turn: Color) -> (i8, Vec<Move>) {
        let (eval, mut idea) = self.evaluate_deep(board, turn, self.depth_limit);
        idea.reverse();
        (eval, idea)
    }

    fn evaluate_deep(&self, board: &BoardState, turn: Color, depth: u8) -> (i8, Vec<Move>) {
        if depth == 0 {
            return (self.naive_evaluation(board, turn), Vec::new());
        }

        let legal_moves = board.get_legal_moves(&turn);

        // No legal moves indicates stalemate or checkmate.
        if legal_moves.len() == 0 {
            let evaluation = if board.is_in_check(&turn) {
                match turn {
                    Color::White => -100,
                    Color::Black => 100,
                }
            } else {
                0
            };

            return (evaluation, Vec::new());
        }

        let mut best_idea = Vec::new();
        let mut best_evaluation = match turn {
            Color::White => -127,
            Color::Black => 127,
        };
        for m in legal_moves {
            let mut next_board = board.clone();
            next_board.apply(m);
            let (evaluation, mut idea) =
                self.evaluate_deep(&next_board, turn.opposite(), depth - 1);
            if turn == Color::White && evaluation > best_evaluation {
                idea.push(m);
                best_idea = idea;
                best_evaluation = evaluation;
            } else if turn == Color::Black && evaluation < best_evaluation {
                idea.push(m);
                best_idea = idea;
                best_evaluation = evaluation;
            }
        }

        (best_evaluation, best_idea)
    }

    pub fn naive_evaluation(&self, board: &BoardState, _turn: Color) -> i8 {
        let mut evaluation = 0;
        for row in (0..8) {
            for col in (0..8) {
                if let Some((color, piece)) = board.get(row, col) {
                    let direction = match color {
                        Color::White => 1,
                        Color::Black => -1,
                    };

                    evaluation += direction
                        * match piece {
                            Piece::Pawn => 1,
                            Piece::Bishop => 3,
                            Piece::Knight => 3,
                            Piece::Rook => 5,
                            Piece::Queen => 9,
                            Piece::King => 0,
                        };
                }
            }
        }
        evaluation
    }
}
