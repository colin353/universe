use chess_engine::Color;

fn main() {
    let mut board = chess_engine::BoardState::new();
    let mut evaluator = chess_engine::Evaluator::new();
    loop {
        println!("{}", board.render(true));

        let mut input = String::new();

        loop {
            match std::io::stdin().read_line(&mut input) {
                Ok(_) => match board.pgn_to_move(Color::White, input.trim()) {
                    Ok(m) => {
                        board.apply(m);
                        break;
                    }
                    Err(msg) => {
                        input = String::new();
                        eprintln!("Error: {}", msg);
                    }
                },
                Err(_) => break,
            }
        }

        println!("{}", board.render(true));

        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(n) => {
                // Do nothing
            }
            Err(_) => break,
        }

        let (eval, idea) = evaluator.evaluate(&board, Color::Black);
        println!("black's plan: {:?} w/ {:?}", eval, idea);
        let m = idea[0];

        println!("black plays {:?}", m);
        board.apply(m);
    }
}
