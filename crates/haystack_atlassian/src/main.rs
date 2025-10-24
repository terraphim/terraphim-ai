use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use std::env;

mod confluence;
mod jira;
#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Confluence related commands
    Confluence {
        #[command(subcommand)]
        command: ConfluenceCommands,
    },
    /// Jira related commands
    Jira {
        #[command(subcommand)]
        command: JiraCommands,
    },
}

#[derive(Subcommand)]
enum ConfluenceCommands {
    /// Search Confluence content using CQL
    Search {
        /// CQL query string
        query: String,
        /// Results limit (1-50)
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },
    /// Get content of a specific Confluence page
    GetPage {
        /// Confluence page ID
        page_id: String,
        /// Include page metadata
        #[arg(short, long, default_value = "true")]
        include_metadata: bool,
    },
    /// Get comments for a specific Confluence page
    GetComments {
        /// Confluence page ID
        page_id: String,
    },
}

#[derive(Subcommand)]
enum JiraCommands {
    /// Get details of a specific Jira issue
    GetIssue {
        /// Jira issue key (e.g., 'PROJ-123')
        issue_key: String,
        /// Fields to expand
        #[arg(short, long)]
        expand: Option<String>,
    },
    /// Search Jira issues using JQL
    Search {
        /// JQL query string
        jql: String,
        /// Comma-separated fields
        #[arg(short, long, default_value = "*all")]
        fields: String,
        /// Results limit (1-50)
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },
    /// Get all issues for a specific Jira project
    GetProjectIssues {
        /// Project key
        project_key: String,
        /// Results limit (1-50)
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let confluence_url = env::var("CONFLUENCE_URL").expect("CONFLUENCE_URL must be set");
    let jira_url = env::var("JIRA_URL").expect("JIRA_URL must be set");
    let username = env::var("ATLASSIAN_USERNAME").expect("ATLASSIAN_USERNAME must be set");
    let token = env::var("ATLASSIAN_TOKEN").expect("ATLASSIAN_TOKEN must be set");

    let cli = Cli::parse();

    match cli.command {
        Commands::Confluence { command } => {
            match command {
                ConfluenceCommands::Search { query, limit } => {
                    let results =
                        confluence::search(&confluence_url, &username, &token, &query, limit)
                            .await?;
                    println!("# Search Results\n");
                    println!(
                        "Found {} pages matching query: `{}`\n",
                        results.len(),
                        query
                    );
                    for result in results {
                        println!("## {}\n", result.title);
                        println!(
                            "**ID:** {} | **Type:** {}{}\n",
                            result.id,
                            result.content_type,
                            result.version.as_ref().map_or("".to_string(), |v| format!(
                                " | **Version:** {}",
                                v.number
                            ))
                        );
                        println!("**Space:** {} ({})\n", result.space.name, result.space.key);

                        if let Some(excerpt) = result.excerpt {
                            println!("**Excerpt:**\n{}\n", excerpt);
                        }

                        println!("**Links:**");
                        println!("- [View in Confluence]({})", result.links.web_ui);
                        if let Some(tiny) = result.links.tinyui {
                            println!("- [Tiny Link]({})", tiny);
                        }
                        if let Some(self_link) = result.links.self_link {
                            println!("- [API Link]({})", self_link);
                        }
                        println!("\n---\n");
                    }
                }
                ConfluenceCommands::GetPage {
                    page_id,
                    include_metadata,
                } => {
                    let page = confluence::get_page(
                        &confluence_url,
                        &username,
                        &token,
                        &page_id,
                        include_metadata,
                    )
                    .await?;
                    println!("# {}\n", page.title);

                    // Metadata section
                    println!("## Metadata\n");
                    println!(
                        "**ID:** {} | **Version:** {}\n",
                        page.id, page.version.number
                    );
                    println!("**Space:** {} ({})\n", page.space.name, page.space.key);

                    if let Some(by) = page.version.by {
                        println!("**Last Modified By:** {}", by.display_name);
                        if let Some(email) = by.email {
                            print!(" ({})", email);
                        }
                        println!("\n");
                    }
                    println!("**Last Modified:** {}\n", page.version.modified);

                    if let Some(message) = page.version.message {
                        println!("**Version Message:** {}\n", message);
                    }

                    // Content section
                    println!("## Content\n");
                    println!("{}\n", page.body.storage.value);

                    // Links section
                    println!("## Links\n");
                    println!("- [View in Confluence]({})", page.links.web_ui);
                    if let Some(tiny) = page.links.tinyui {
                        println!("- [Tiny Link]({})", tiny);
                    }
                    if let Some(self_link) = page.links.self_link {
                        println!("- [API Link]({})", self_link);
                    }
                    println!();
                }
                ConfluenceCommands::GetComments { page_id } => {
                    let comments =
                        confluence::get_comments(&confluence_url, &username, &token, &page_id)
                            .await?;
                    println!("# Comments\n");
                    println!("Found {} comments\n", comments.len());
                    for comment in comments {
                        println!("## Comment by {}\n", comment.author.display_name);

                        if let Some(email) = comment.author.email {
                            println!("**Author Email:** {}\n", email);
                        }

                        if let Some(created) = comment.created_date {
                            println!("**Created:** {}", created);
                        }
                        if let Some(updated) = comment.updated_date {
                            println!(" | **Updated:** {}", updated);
                        }
                        println!("\n");

                        println!("{}\n", comment.body.storage.value);

                        println!(
                            "**Version:** {} | **Minor Edit:** {}\n",
                            comment.version.number,
                            comment.version.minor_edit.unwrap_or(false)
                        );

                        if let Some(msg) = comment.version.message {
                            println!("**Version Message:** {}\n", msg);
                        }

                        println!("---\n");
                    }
                }
            }
        }
        Commands::Jira { command } => match command {
            JiraCommands::GetIssue { issue_key, expand } => {
                let issue =
                    jira::get_issue(&jira_url, &username, &token, &issue_key, expand.as_deref())
                        .await?;
                println!("# Issue {}\n", issue.key);
                println!("## Summary\n{}\n", issue.fields.summary);
                println!(
                    "**Type:** {} | **Status:** {} | **Priority:** {}\n",
                    issue.fields.issue_type.name,
                    issue.fields.status.name,
                    issue.fields.priority.as_ref().map_or("None", |p| &p.name)
                );

                if let Some(desc) = &issue.fields.description {
                    println!("## Description\n{}\n", desc);
                }

                if let Some(resolution) = &issue.fields.resolution {
                    println!("## Resolution\n{}\n", resolution.name);
                }

                if let Some(assignee) = &issue.fields.assignee {
                    println!(
                        "## Assignee\n{}{}\n",
                        assignee.display_name,
                        assignee
                            .email_address
                            .as_ref()
                            .map_or("".to_string(), |e| format!(" ({})", e))
                    );
                }

                if let Some(reporter) = &issue.fields.reporter {
                    println!(
                        "## Reporter\n{}{}\n",
                        reporter.display_name,
                        reporter
                            .email_address
                            .as_ref()
                            .map_or("".to_string(), |e| format!(" ({})", e))
                    );
                }

                if let Some(components) = &issue.fields.components {
                    if !components.is_empty() {
                        println!(
                            "## Components\n{}\n",
                            components
                                .iter()
                                .map(|c| c.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                }

                if let Some(versions) = &issue.fields.fix_versions {
                    if !versions.is_empty() {
                        println!(
                            "## Fix Versions\n{}\n",
                            versions
                                .iter()
                                .map(|v| v.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                }

                if let Some(labels) = &issue.fields.labels {
                    if !labels.is_empty() {
                        println!("## Labels\n{}\n", labels.join(", "));
                    }
                }

                if let Some(due_date) = &issue.fields.due_date {
                    println!("## Due Date\n{}\n", due_date);
                }

                println!("## Dates\n");
                println!("- **Created:** {}", issue.fields.created);
                println!("- **Updated:** {}\n", issue.fields.updated);

                println!(
                    "[View in Jira]({}/browse/{})\n",
                    jira_url.trim_end_matches('/'),
                    issue.key
                );
            }
            JiraCommands::Search {
                jql,
                fields: _,
                limit,
            } => {
                let results =
                    jira::search(&jira_url, &username, &token, &jql, "*all", limit).await?;
                println!("# Search Results\n");
                println!("Found {} issues matching query: `{}`\n", results.len(), jql);
                for issue in results {
                    println!("## {} - {}\n", issue.key, issue.fields.summary);
                    println!(
                        "**Type:** {} | **Status:** {} | **Priority:** {}",
                        issue.fields.issue_type.name,
                        issue.fields.status.name,
                        issue.fields.priority.as_ref().map_or("None", |p| &p.name)
                    );

                    if let Some(assignee) = &issue.fields.assignee {
                        println!(" | **Assignee:** {}", assignee.display_name);
                    }
                    println!("\n");

                    if let Some(desc) = &issue.fields.description {
                        if desc.len() > 200 {
                            println!("{:.200}...\n", desc);
                        } else {
                            println!("{}\n", desc);
                        }
                    }

                    if let Some(components) = &issue.fields.components {
                        if !components.is_empty() {
                            println!(
                                "**Components:** {}",
                                components
                                    .iter()
                                    .map(|c| c.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                        }
                    }

                    if let Some(labels) = &issue.fields.labels {
                        if !labels.is_empty() {
                            println!("**Labels:** {}", labels.join(", "));
                        }
                    }
                    println!("\n");

                    println!(
                        "*Created: {} | Updated: {}*\n",
                        issue.fields.created, issue.fields.updated
                    );

                    println!(
                        "[View in Jira]({}/browse/{})\n",
                        jira_url.trim_end_matches('/'),
                        issue.key
                    );
                    println!("---\n");
                }
            }
            JiraCommands::GetProjectIssues { project_key, limit } => {
                let issues =
                    jira::get_project_issues(&jira_url, &username, &token, &project_key, limit)
                        .await?;
                println!("# Project {} Issues\n", project_key);
                println!("Found {} issues\n", issues.len());
                for issue in issues {
                    println!("## {} - {}\n", issue.key, issue.fields.summary);
                    println!(
                        "**Type:** {} | **Status:** {} | **Priority:** {}",
                        issue.fields.issue_type.name,
                        issue.fields.status.name,
                        issue.fields.priority.as_ref().map_or("None", |p| &p.name)
                    );

                    if let Some(assignee) = &issue.fields.assignee {
                        println!(" | **Assignee:** {}", assignee.display_name);
                    }
                    println!("\n");

                    if let Some(desc) = &issue.fields.description {
                        if desc.len() > 200 {
                            println!("{:.200}...\n", desc);
                        } else {
                            println!("{}\n", desc);
                        }
                    }

                    if let Some(components) = &issue.fields.components {
                        if !components.is_empty() {
                            println!(
                                "**Components:** {}",
                                components
                                    .iter()
                                    .map(|c| c.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                        }
                    }

                    if let Some(labels) = &issue.fields.labels {
                        if !labels.is_empty() {
                            println!("**Labels:** {}", labels.join(", "));
                        }
                    }
                    println!("\n");

                    println!(
                        "*Created: {} | Updated: {}*\n",
                        issue.fields.created, issue.fields.updated
                    );

                    println!(
                        "[View in Jira]({}/browse/{})\n",
                        jira_url.trim_end_matches('/'),
                        issue.key
                    );
                    println!("---\n");
                }
            }
        },
    }

    Ok(())
}
