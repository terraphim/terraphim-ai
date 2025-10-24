use anyhow::Result;
use clap::Parser;
use jmap_client::JMAPClient;

/// Command line arguments
#[derive(Parser)]
struct CommandLineArgs {
    /// The search query string
    query: String,

    /// The output format (json or markdown)
    #[arg(long, default_value = "markdown")]
    output_format: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the command line arguments
    let command_line_args = CommandLineArgs::parse();

    // Get the FASTMAIL_API_TOKEN from environment variables
    let fastmail_api_token = std::env::var("FASTMAIL_API_TOKEN")?;

    // Create a new JMAP client
    let jmap_client = JMAPClient::new(fastmail_api_token).await?;

    // Search for emails using the provided query
    let matching_emails = jmap_client.search_emails(&command_line_args.query).await?;

    // Print the matching emails in the requested output format
    match command_line_args.output_format.as_str() {
        "json" => {
            // Print emails in JSON format
            println!("{}", serde_json::to_string_pretty(&matching_emails)?);
        }
        _ => {
            // Print emails in markdown format
            for email in matching_emails {
                println!("## Email: {}", email.subject.unwrap_or_default());

                if let Some(from_addresses) = email.from {
                    for sender in from_addresses {
                        println!(
                            "**From:** {} <{}>",
                            sender.name.unwrap_or_default(),
                            sender.email
                        );
                    }
                }

                println!("**Received:** {}", email.received_at.unwrap_or_default());
                println!("\n### Content:\n");

                for body_part in &email.text_body {
                    if let Some(body_value) = email.body_values.get(&body_part.part_id) {
                        for paragraph in body_value.value.split('\n') {
                            if !paragraph.trim().is_empty() {
                                println!("{}\n", paragraph.trim());
                            }
                        }
                    }
                }
                println!("---\n");
            }
        }
    }

    Ok(())
}
