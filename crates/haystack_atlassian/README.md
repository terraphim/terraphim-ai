# Atlassian Haystack CLI

A command-line interface for searching and retrieving content from Confluence and Jira.

## Installation

1. Make sure you have Rust installed. If not, install it from [rustup.rs](https://rustup.rs/)
2. Clone this repository
3. Build the project:
```bash
cargo build --release
```

## Configuration

Create a `.env` file in the project root with your Atlassian credentials:

```env
CONFLUENCE_URL=https://your-domain.atlassian.net
JIRA_URL=https://your-domain.atlassian.net
ATLASSIAN_USERNAME=your-email@example.com
ATLASSIAN_TOKEN=your-api-token
```

To get an API token:
1. Log in to https://id.atlassian.com/manage/api-tokens
2. Click "Create API token"
3. Copy the token and paste it in your `.env` file

## Usage

### Confluence Commands

#### Search Confluence Content
```bash
# Basic text search
cargo run -- confluence search "text ~ \"search term\""

# Search in specific space
cargo run -- confluence search "space = \"TEAM\" AND text ~ \"search term\""

# Combine multiple conditions
cargo run -- confluence search "space = \"TEAM\" AND text ~ \"search term\" AND type = \"page\""

# Limit results
cargo run -- confluence search --limit 5 "text ~ \"search term\""
```

#### Get Confluence Page Content
```bash
# Get page with metadata
cargo run -- confluence get-page --page-id "123456"

# Get page without metadata
cargo run -- confluence get-page --page-id "123456" --include-metadata false
```

#### Get Page Comments
```bash
# Get all comments for a page
cargo run -- confluence get-comments --page-id "123456"
```

### Jira Commands

#### Search Jira Issues
```bash
# Basic JQL search
cargo run -- jira search "project = PROJ AND status = 'In Progress'"

# Search with custom fields
cargo run -- jira search --fields "summary,description,status" "project = PROJ"

# Limit results
cargo run -- jira search --limit 5 "project = PROJ"
```

#### Get Single Issue
```bash
# Get issue details
cargo run -- jira get-issue --issue-key "PROJ-123"

# Get issue with expanded fields
cargo run -- jira get-issue --issue-key "PROJ-123" --expand "changelog"
```

#### Get Project Issues
```bash
# Get all issues from a project
cargo run -- jira get-project-issues --project-key "PROJ"

# Limit number of issues
cargo run -- jira get-project-issues --project-key "PROJ" --limit 5
```

## Search Query Syntax

### Confluence CQL Examples
- `text ~ "search term"` - Full-text search
- `space = "TEAM"` - Search in specific space
- `type = "page"` - Filter by content type
- `lastmodified >= "2023-01-01"` - Time-based search
- `creator = "username"` - Search by creator
- `label = "important"` - Search by label

### Jira JQL Examples
- `project = "PROJ"` - Search in project
- `status = "In Progress"` - Filter by status
- `assignee = currentUser()` - Assigned to current user
- `created >= -30d` - Created in last 30 days
- `priority = High` - Filter by priority
- `labels = "bug"` - Search by label

## Error Handling

The CLI will display meaningful error messages for common issues:
- Invalid credentials
- Network connectivity problems
- Invalid search syntax
- Missing or malformed parameters

## Contributing

1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request
