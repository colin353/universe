use chess_engine::Color;

fn main() {
    let mut board = chess_engine::BoardState::new();
    loop {
        println!("{}", board.render());

        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(n) => {
                // Do nothing
            }
            Err(_) => break,
        }

        let moves = board.get_legal_moves(&Color::White);
        let m = moves[rand::random::<usize>() % moves.len()];
        println!("white plays {:?}", m);
        board.apply(m);

        println!("{}", board.render());

        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(n) => {
                // Do nothing
            }
            Err(_) => break,
        }
        let moves = board.get_legal_moves(&Color::Black);
        let m = moves[rand::random::<usize>() % moves.len()];
        println!("black plays {:?}", m);
        board.apply(m);
    }
}
