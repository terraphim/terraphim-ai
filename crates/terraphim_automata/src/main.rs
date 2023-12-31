use std::collections::HashMap;
use std::error::Error;

use terraphim_automata::load_automata;
use terraphim_automata::matcher::{find_matches, Dictionary};

pub type ResponseJSON = HashMap<String, Dictionary>;

fn main() -> Result<(), Box<dyn Error>> {
    let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    // let automata_url= "./data/term_to_id.json"
    let dict_hash = load_automata(automata_url).unwrap();
    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, dict_hash, true)?;
    println!("Matches: {:?}", matches);
    let automata_url = "./data/term_to_id.json";
    let dict_hash = load_automata(automata_url).unwrap();
    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, dict_hash, true)?;
    println!("Matches: {:?}", matches);
    Ok(())
}
