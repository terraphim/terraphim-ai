//! Liquid-compatible prompt template rendering.
//!
//! Renders the workflow prompt body with `issue` and `attempt` variables.
//! Uses strict mode: unknown variables and filters produce errors.

use crate::error::{Result, SymphonyError};
use crate::tracker::Issue;

/// Default fallback prompt when the workflow body is empty.
const DEFAULT_PROMPT: &str = "You are working on an issue from the configured tracker.";

/// Render the prompt template with issue context.
///
/// # Arguments
/// * `template_str` - Liquid template string (the workflow prompt body).
/// * `issue` - The normalised issue to render into the template.
/// * `attempt` - Retry/continuation attempt number. `None` for first run.
///
/// # Errors
/// Returns `TemplateParseError` if the template syntax is invalid, or
/// `TemplateRenderError` if a referenced variable or filter is unknown.
pub fn render_prompt(
    template_str: &str,
    issue: &Issue,
    attempt: Option<u32>,
) -> Result<String> {
    let source = if template_str.trim().is_empty() {
        DEFAULT_PROMPT
    } else {
        template_str
    };

    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .map_err(|e| SymphonyError::TemplateParseError {
            reason: e.to_string(),
        })?
        .parse(source)
        .map_err(|e| SymphonyError::TemplateParseError {
            reason: e.to_string(),
        })?;

    let issue_obj = issue_to_liquid_object(issue);

    let mut globals = liquid::Object::new();
    globals.insert("issue".into(), liquid::model::Value::Object(issue_obj));

    match attempt {
        Some(n) => {
            globals.insert(
                "attempt".into(),
                liquid::model::Value::Scalar(liquid::model::Scalar::new(n as i64)),
            );
        }
        None => {
            globals.insert("attempt".into(), liquid::model::Value::Nil);
        }
    }

    template
        .render(&globals)
        .map_err(|e| SymphonyError::TemplateRenderError {
            reason: e.to_string(),
        })
}

/// Convert an Issue into a Liquid object for template rendering.
fn issue_to_liquid_object(issue: &Issue) -> liquid::Object {
    let mut obj = liquid::Object::new();

    obj.insert(
        "id".into(),
        liquid::model::Value::Scalar(issue.id.clone().into()),
    );
    obj.insert(
        "identifier".into(),
        liquid::model::Value::Scalar(issue.identifier.clone().into()),
    );
    obj.insert(
        "title".into(),
        liquid::model::Value::Scalar(issue.title.clone().into()),
    );
    obj.insert(
        "description".into(),
        match &issue.description {
            Some(d) => liquid::model::Value::Scalar(d.clone().into()),
            None => liquid::model::Value::Nil,
        },
    );
    obj.insert(
        "priority".into(),
        match issue.priority {
            Some(p) => liquid::model::Value::Scalar(liquid::model::Scalar::new(p as i64)),
            None => liquid::model::Value::Nil,
        },
    );
    obj.insert(
        "state".into(),
        liquid::model::Value::Scalar(issue.state.clone().into()),
    );
    obj.insert(
        "branch_name".into(),
        match &issue.branch_name {
            Some(b) => liquid::model::Value::Scalar(b.clone().into()),
            None => liquid::model::Value::Nil,
        },
    );
    obj.insert(
        "url".into(),
        match &issue.url {
            Some(u) => liquid::model::Value::Scalar(u.clone().into()),
            None => liquid::model::Value::Nil,
        },
    );

    let labels: Vec<liquid::model::Value> = issue
        .labels
        .iter()
        .map(|l| liquid::model::Value::Scalar(l.clone().into()))
        .collect();
    obj.insert("labels".into(), liquid::model::Value::Array(labels));

    let blockers: Vec<liquid::model::Value> = issue
        .blocked_by
        .iter()
        .map(|b| {
            let mut bobj = liquid::Object::new();
            bobj.insert(
                "id".into(),
                match &b.id {
                    Some(id) => liquid::model::Value::Scalar(id.clone().into()),
                    None => liquid::model::Value::Nil,
                },
            );
            bobj.insert(
                "identifier".into(),
                match &b.identifier {
                    Some(ident) => liquid::model::Value::Scalar(ident.clone().into()),
                    None => liquid::model::Value::Nil,
                },
            );
            bobj.insert(
                "state".into(),
                match &b.state {
                    Some(s) => liquid::model::Value::Scalar(s.clone().into()),
                    None => liquid::model::Value::Nil,
                },
            );
            liquid::model::Value::Object(bobj)
        })
        .collect();
    obj.insert("blocked_by".into(), liquid::model::Value::Array(blockers));

    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_issue() -> Issue {
        Issue {
            id: "abc123".into(),
            identifier: "MT-42".into(),
            title: "Fix the widget".into(),
            description: Some("The widget is broken.".into()),
            priority: Some(1),
            state: "Todo".into(),
            branch_name: None,
            url: Some("https://tracker.example.com/MT-42".into()),
            labels: vec!["bug".into(), "urgent".into()],
            blocked_by: vec![],
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }

    #[test]
    fn render_basic_template() {
        let template = "Working on {{ issue.identifier }}: {{ issue.title }}";
        let result = render_prompt(template, &sample_issue(), None).unwrap();
        assert_eq!(result, "Working on MT-42: Fix the widget");
    }

    #[test]
    fn render_with_attempt() {
        let template = "{% if attempt %}Retry attempt {{ attempt }}{% else %}First run{% endif %}";
        let first = render_prompt(template, &sample_issue(), None).unwrap();
        assert_eq!(first, "First run");

        let retry = render_prompt(template, &sample_issue(), Some(2)).unwrap();
        assert_eq!(retry, "Retry attempt 2");
    }

    #[test]
    fn render_with_labels() {
        let template = "Labels: {% for label in issue.labels %}{{ label }} {% endfor %}";
        let result = render_prompt(template, &sample_issue(), None).unwrap();
        assert!(result.contains("bug"));
        assert!(result.contains("urgent"));
    }

    #[test]
    fn render_empty_template_uses_default() {
        let result = render_prompt("", &sample_issue(), None).unwrap();
        assert_eq!(result, DEFAULT_PROMPT);
    }

    #[test]
    fn render_whitespace_only_template_uses_default() {
        let result = render_prompt("   \n  ", &sample_issue(), None).unwrap();
        assert_eq!(result, DEFAULT_PROMPT);
    }

    #[test]
    fn render_with_description() {
        let template = "Description: {{ issue.description }}";
        let result = render_prompt(template, &sample_issue(), None).unwrap();
        assert_eq!(result, "Description: The widget is broken.");
    }

    #[test]
    fn render_nil_description() {
        let mut issue = sample_issue();
        issue.description = None;
        let template = "{% if issue.description %}Has desc{% else %}No desc{% endif %}";
        let result = render_prompt(template, &issue, None).unwrap();
        assert_eq!(result, "No desc");
    }
}
