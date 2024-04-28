use terraphim_automata::matcher::find_matches;
use terraphim_automata::Result;
use terraphim_automata::{load_thesaurus, AutomataPath};

#[tokio::main]
async fn main() -> Result<()> {
    // let automata_url= "./data/term_to_id.json"
    let thesaurus = load_thesaurus(&AutomataPath::remote_example()).await?;
    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, thesaurus, true)?;
    println!("Matches: {:?}", matches);

    // Create the URL from the absolute file path
    let automata_path = AutomataPath::from_local("./data/term_to_id.json");
    let thesaurus = load_thesaurus(&automata_path).await?;

    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, thesaurus, true)?;
    println!("Matches: {:?}", matches);
    Ok(())
}
