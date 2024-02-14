#![no_main]
curta_zkvm::entrypoint!(main);

use chess::{Board, ChessMove};
use std::str::FromStr;

pub fn main() {
    // Read the board position in Forsyth-Edwards Notation (FEN), and a move in Standard Algebraic Notation (SAN)
    let fen = curta_zkvm::io::read::<String>();
    let san = curta_zkvm::io::read::<String>();

    // Generate the chessboard from the FEN input
    let b = Board::from_str(&fen).expect("valid FEN board");

    // Try to parse the SAN as a legal chess move
    let is_valid_move = match ChessMove::from_san(&b, &san) {
        Ok(_) => true,
        Err(_) => false
    };

    // Write whether or not the move is legal
    curta_zkvm::io::write(&is_valid_move);
}
