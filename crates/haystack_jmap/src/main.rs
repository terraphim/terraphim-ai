use anyhow::Result;
use clap::Parser;
use haystack_jmap::JMAPClient;

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
    env_logger::init();

    let command_line_args = CommandLineArgs::parse();

    let access_token =
        std::env::var("JMAP_ACCESS_TOKEN").or_else(|_| std::env::var("FASTMAIL_API_TOKEN"))?;

    let session_url = std::env::var("JMAP_SESSION_URL")
        .unwrap_or_else(|_| "https://api.fastmail.com/jmap/session".to_string());

    let jmap_client = JMAPClient::new(access_token, &session_url).await?;

    let matching_emails = jmap_client
        .search_emails(&command_line_args.query, 50)
        .await?;

    match command_line_args.output_format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&matching_emails)?);
        }
        _ => {
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
