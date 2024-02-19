use terraphim_automata::load_automata;
use terraphim_automata::matcher::find_matches;
terraphim_types::NormalizedTerm;
use terraphim_automata::Result;

use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    // let automata_url= "./data/term_to_id.json"
    let thesaurus = load_automata(automata_url).await?;
    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, thesaurus, true)?;
    println!("Matches: {:?}", matches);
    let automata_url = "./data/term_to_id.json";
    let thesaurus = load_automata(automata_url).await?;
    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, thesaurus, true)?;
    println!("Matches: {:?}", matches);
    Ok(())
}
