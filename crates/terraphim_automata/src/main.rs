use terraphim_automata::load_thesaurus;
use terraphim_automata::matcher::find_matches;
use terraphim_automata::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    // let automata_url= "./data/term_to_id.json"
    let thesaurus = load_thesaurus(automata_url).await?;
    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, thesaurus, true)?;
    println!("Matches: {:?}", matches);
    let automata_url = "./data/term_to_id.json";
    let thesaurus = load_thesaurus(automata_url).await?;
    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, thesaurus, true)?;
    println!("Matches: {:?}", matches);
    Ok(())
}
