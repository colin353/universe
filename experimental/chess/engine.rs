pub use chess::*;

const DEPTH_LIMIT: u8 = 6;

pub struct Evaluator {
    depth_limit: u8,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            depth_limit: DEPTH_LIMIT,
        }
    }

    fn center_preference(p: Position) -> i8 {
        std::cmp::min(p.row(), p.col())
    }

    fn preranking(m: &Move) -> i8 {
        match m {
            Move::Position(_, p, start, end) => {
                let pref = Evaluator::center_preference(*end);
                -pref
                    + if *p == Piece::Pawn {
                        if (end.row() - start.row()).abs() == 2 {
                            -5
                        } else {
                            0
                        }
                    } else {
                        0
                    }
            }
            Move::CastleQueenside(_) => -8,
            Move::CastleKingside(_) => -10,
            Move::Takes(_, p, _, end) => {
                -25 + match p {
                    // Discount the value of the piece taking. This means that
                    // super expensive captures (e.g. queen captures) go last.
                    Piece::Pawn => 0,
                    Piece::Bishop => 6,
                    Piece::Knight => 6,
                    Piece::Rook => 10,
                    Piece::Queen => 18,
                    Piece::King => 30,
                }
            }
            Move::Promotion(_, _, _, Piece::Queen) => -100,
            Move::Promotion(_, _, _, _) => -50,
        }
    }

    pub fn sort_moves(&self, moves: &mut Vec<Move>) {
        moves.sort_by_key(Evaluator::preranking);
    }

    pub fn evaluate(&self, board: &BoardState, turn: Color) -> (i8, Vec<Move>) {
        let start = std::time::Instant::now();
        let (eval, mut idea, possibilities) =
            self.evaluate_deep(board, turn, -127, 127, self.depth_limit);
        idea.reverse();
        println!(
            "evaluated {} possibilities in {} ms",
            possibilities,
            start.elapsed().as_millis()
        );
        (eval, idea)
    }

    fn evaluate_deep(
        &self,
        board: &BoardState,
        turn: Color,
        mut alpha: i8,
        mut beta: i8,
        depth: u8,
    ) -> (i8, Vec<Move>, u64) {
        if depth == 0 {
            return (self.naive_evaluation(board, turn), Vec::new(), 1);
        }

        let mut legal_moves = board.get_legal_moves(&turn);
        self.sort_moves(&mut legal_moves);

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

            return (evaluation, Vec::new(), 1);
        }

        let mut best_idea = Vec::new();
        let mut best_evaluation = match turn {
            Color::White => -127,
            Color::Black => 127,
        };
        let mut next_board = BoardState::new();
        let mut possibilities = 0;
        for m in legal_moves {
            next_board.clone_from(board);
            next_board.apply(m);
            let (evaluation, mut idea, boards_evaluated) =
                self.evaluate_deep(&next_board, turn.opposite(), alpha, beta, depth - 1);
            possibilities += boards_evaluated;
            if turn == Color::White {
                if evaluation > best_evaluation {
                    best_evaluation = evaluation;
                    idea.push(m);
                    best_idea = idea.clone();
                }
                if evaluation > alpha {
                    alpha = evaluation;
                }
                if beta <= alpha {
                    break;
                }
            } else {
                if evaluation < best_evaluation {
                    idea.push(m);
                    best_idea = idea.clone();
                    best_evaluation = evaluation;
                }

                if evaluation < beta {
                    beta = evaluation;
                }
                if beta <= alpha {
                    break;
                }
            }
        }

        (best_evaluation, best_idea, possibilities)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_finds_checkmate() {
        let b = BoardState::from_str(
            "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        │ │ │ │ │ │ │♖│ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │♛│♟│ │♟│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♟│♟│♜│ │♞│ │♟│♚│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │♙│ │ │ │ │♕│♙│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │♘│ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♙│ │ │ │ │♙│♙│ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │ │♔│ │\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘",
        );

        let evaluator = Evaluator::new();
        let (evaluation, idea) = evaluator.evaluate(&b, Color::White);
        assert_eq!(&render_idea(&idea), "□ Qh5 ⬛gxh5 □ Nf5");
    }
}
