use chess::*;

pub struct RecordedGame<'a> {
    board: BoardState,
    moves: Vec<&'a str>,
    current_move: usize,
}

impl<'a> RecordedGame<'a> {
    pub fn from_str(pgn: &'a str) -> Self {
        Self {
            board: BoardState::new(),
            moves: Self::parse_moves(pgn),
            current_move: 0,
        }
    }

    fn parse_moves(pgn: &'a str) -> Vec<&'a str> {
        let mut output = Vec::new();
        for line in pgn
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.starts_with("[") && !l.starts_with(";") && !l.is_empty())
        {
            let components = line.split("{");
            for component in components {
                let mut comment_parts = component.split("}");
                let mut moves = match comment_parts.next() {
                    Some(m) => m,
                    None => "",
                };
                moves = match comment_parts.next() {
                    Some(m) => m,
                    None => moves,
                };
                Self::extract_moves(moves, &mut output);
            }
        }
        output
    }

    fn extract_moves(pgn: &'a str, out: &mut Vec<&'a str>) {
        for m in pgn.split(" ").map(|m| m.trim()) {
            if let Some(ch) = m.chars().next() {
                if ch.is_numeric() {
                    continue;
                }
            }

            if m.trim().is_empty() {
                continue;
            }

            out.push(m.trim());
        }
    }
}

impl<'a> Iterator for RecordedGame<'a> {
    type Item = BoardState;

    fn next(&mut self) -> Option<Self::Item> {
        let color = match self.current_move % 2 {
            0 => Color::White,
            _ => Color::Black,
        };

        if self.current_move >= self.moves.len() {
            return None;
        }

        self.board.apply_pgn(color, self.moves[self.current_move]);
        self.current_move += 1;

        Some(self.board.clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_pgn() {
        let input = r#"
[Event "F/S Return Match"]
[Site "Belgrade, Serbia JUG"]
[Date "1992.11.04"]
[Round "29"]
[White "Fischer, Robert J."]
[Black "Spassky, Boris V."]
[Result "1/2-1/2"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 {This opening is called the Ruy Lopez.}
4. Ba4 Nf6 5. O-O Be7 6. Re1 b5 7. Bb3 d6 8. c3 O-O 9. h3 Nb8 10. d4 Nbd7
11. c4 c6 12. cxb5 axb5 13. Nc3 Bb7 14. Bg5 b4 15. Nb1 h6 16. Bh4 c5 17. dxe5
Nxe4 18. Bxe7 Qxe7 19. exd6 Qf6 20. Nbd2 Nxd6 21. Nc4 Nxc4 22. Bxc4 Nb6
23. Ne5 Rae8 24. Bxf7+ Rxf7 25. Nxf7 Rxe1+ 26. Qxe1 Kxf7 27. Qe3 Qg5 28. Qxg5
hxg5 29. b3 Ke6 30. a3 Kd6 31. axb4 cxb4 32. Ra5 Nd5 33. f3 Bc8 34. Kf2 Bf5
35. Ra7 g6 36. Ra6+ Kc5 37. Ke1 Nf4 38. g3 Nxh3 39. Kd2 Kb5 40. Rd6 Kc5 41. Ra6
Nf2 42. g4 Bd3 43. Re6 1/2-1/2
"#;

        let moves = RecordedGame::parse_moves(input);
        assert_eq!(moves[0], "e4");
        assert_eq!(moves[1], "e5");
        assert_eq!(moves[2], "Nf3");
        assert_eq!(moves[3], "Nc6");
        assert_eq!(moves[4], "Bb5");
        assert_eq!(moves[5], "a6");
        assert_eq!(moves[6], "Ba4");
    }

    #[test]
    fn test_iterate_board() {
        let input = r#"
[Event "F/S Return Match"]
[Site "Belgrade, Serbia JUG"]
[Date "1992.11.04"]
[Round "29"]
[White "Fischer, Robert J."]
[Black "Spassky, Boris V."]
[Result "1/2-1/2"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 {This opening is called the Ruy Lopez.}
4. Ba4 Nf6 5. O-O Be7 6. Re1 b5 7. Bb3 d6 8. c3 O-O 9. h3 Nb8 10. d4 Nbd7
11. c4 c6 12. cxb5 axb5 13. Nc3 Bb7 14. Bg5 b4 15. Nb1 h6 16. Bh4 c5 17. dxe5
Nxe4 18. Bxe7 Qxe7 19. exd6 Qf6 20. Nbd2 Nxd6 21. Nc4 Nxc4 22. Bxc4 Nb6
23. Ne5 Rae8 24. Bxf7+ Rxf7 25. Nxf7 Rxe1+ 26. Qxe1 Kxf7 27. Qe3 Qg5 28. Qxg5
hxg5 29. b3 Ke6 30. a3 Kd6 31. axb4 cxb4 32. Ra5 Nd5 33. f3 Bc8 34. Kf2 Bf5
35. Ra7 g6 36. Ra6+ Kc5 37. Ke1 Nf4 38. g3 Nxh3 39. Kd2 Kb5 40. Rd6 Kc5 41. Ra6
Nf2 42. g4 Bd3 43. Re6 1/2-1/2
"#;

        let mut game = RecordedGame::from_str(input);
        let b = game.skip(5).next().unwrap();

        println!("{}", b.render(true));

        let expected = "\
        ┌─┬─┬─┬─┬─┬─┬─┬─┐\n\
        │♜│ │♝│♛│♚│♝│♞│♜│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │♟│♟│♟│ │♟│♟│♟│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♟│ │♞│ │ │ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │♗│ │ │♟│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │♙│ │ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │ │ │ │ │ │♘│ │ │\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♙│♙│♙│♙│ │♙│♙│♙│\n\
        ├─┼─┼─┼─┼─┼─┼─┼─┤\n\
        │♖│♘│♗│♕│♔│ │ │♖│\n\
        └─┴─┴─┴─┴─┴─┴─┴─┘";

        assert_eq!(b.render(false), expected);

        let mut game = RecordedGame::from_str(input);
        for position in game {
            println!("{}", b.render(true));
        }
    }
}
