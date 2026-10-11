use crate::banner::Degradation;
use crate::profile::Profile;
use crate::types::WriterOutput;

/// XML-escapes text for safe inclusion within XML elements.
pub fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Renders a WriterOutput into the standardized XML prompt format.
pub fn render(
    output: &WriterOutput,
    profile: &Profile,
    degradation: Option<&Degradation>,
) -> String {
    let mut sections = Vec::new();

    // 1. Degradation banner (if present)
    if let Some(deg) = degradation {
        sections.push(deg.render_banner());
    }

    // 2. <task>
    let task_escaped = xml_escape(output.task.trim());
    sections.push(format!("<task>\n{}\n</task>", task_escaped));

    // 3. <context> (only if non-empty)
    if !output.context.is_empty() {
        let items: Vec<String> = output
            .context
            .iter()
            .map(|c| {
                let trimmed = c.trim();
                let escaped = xml_escape(trimmed);
                if escaped.starts_with("- ") || escaped.starts_with("* ") {
                    escaped
                } else {
                    format!("- {}", escaped)
                }
            })
            .collect();
        sections.push(format!("<context>\n{}\n</context>", items.join("\n")));
    }

    // 4. <acceptance_criteria>
    let ac_items: Vec<String> = output
        .acceptance_criteria
        .iter()
        .map(|a| {
            let trimmed = a.trim();
            let escaped = xml_escape(trimmed);
            if escaped.starts_with("- ") || escaped.starts_with("* ") {
                escaped
            } else {
                format!("- {}", escaped)
            }
        })
        .collect();
    if !ac_items.is_empty() {
        sections.push(format!(
            "<acceptance_criteria>\n{}\n</acceptance_criteria>",
            ac_items.join("\n")
        ));
    }

    // 5. <rules> (fixed profile rules + extra_rules)
    let mut rules_content = profile.rules.trim().to_string();
    if !output.extra_rules.is_empty() {
        for rule in &output.extra_rules {
            let trimmed = rule.trim();
            if trimmed.is_empty() {
                continue;
            }
            let escaped = xml_escape(trimmed);
            if escaped.starts_with("- ") || escaped.starts_with("* ") {
                rules_content.push_str(&format!("\n{}", escaped));
            } else {
                rules_content.push_str(&format!("\n- {}", escaped));
            }
        }
    }
    sections.push(format!("<rules>\n{}\n</rules>", rules_content));

    // 6. <report> (fixed profile report + extra_report)
    let mut report_content = profile.report.trim().to_string();
    if !output.extra_report.is_empty() {
        for rep in &output.extra_report {
            let trimmed = rep.trim();
            if trimmed.is_empty() {
                continue;
            }
            let escaped = xml_escape(trimmed);
            report_content.push_str(&format!("\n{}", escaped));
        }
    }
    sections.push(format!("<report>\n{}\n</report>", report_content));

    sections.join("\n\n")
}
