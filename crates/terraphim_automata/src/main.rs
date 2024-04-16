use std::env;

use terraphim_automata::load_thesaurus;
use terraphim_automata::matcher::find_matches;
use terraphim_automata::Result;
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    let automata_url =
        Url::parse("https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json").unwrap();
    // let automata_url= "./data/term_to_id.json"
    let thesaurus = load_thesaurus(automata_url).await?;
    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, thesaurus, true)?;
    println!("Matches: {:?}", matches);

    // Create the URL from the absolute file path
    let automata_path = "./data/term_to_id.json";
    let base_path = env::current_dir().expect("Failed to get current directory");
    let absolute_automata_path = base_path.join(automata_path);
    let automata_url = Url::from_file_path(absolute_automata_path).unwrap();

    let thesaurus = load_thesaurus(automata_url).await?;
    let haystack =
        "I am a text with the word Organization strategic plan and bar and project calendar";
    let matches = find_matches(haystack, thesaurus, true)?;
    println!("Matches: {:?}", matches);
    Ok(())
}
