use crate::banner::Degradation;
use crate::profile::Profile;
use crate::types::{ItemConfidence, WriterOutput};

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

/// Adds confidence marker prefix if score is below threshold or None.
fn confidence_marker(score: Option<f64>, threshold: f64) -> String {
    match score {
        Some(s) if s < threshold => "??[low confidence] ".to_string(),
        None => "??[unchecked] ".to_string(),
        _ => String::new(),
    }
}

/// Renders a WriterOutput into the standardized XML prompt format.
pub fn render(
    output: &WriterOutput,
    profile: &Profile,
    degradation: Option<&Degradation>,
    confidence: Option<&[ItemConfidence]>,
    threshold: f64,
) -> String {
    let mut sections = Vec::new();

    // 1. Degradation banner (if present)
    if let Some(deg) = degradation {
        sections.push(deg.render_banner());
    }

    // 2. Confidence banner (if any low confidence or unchecked items)
    if let Some(conf_items) = confidence {
        let low_conf_sections: Vec<String> = conf_items
            .iter()
            .filter_map(|ic| {
                if ic.score.is_none() || ic.score.unwrap_or(1.0) < threshold {
                    Some(ic.section.clone())
                } else {
                    None
                }
            })
            .collect();
        if !low_conf_sections.is_empty() {
            let notice = format!(
                "?? PFMT REVIEW: low confidence in: {}. Check marked lines before sending.",
                low_conf_sections.join(", ")
            );
            sections.push(notice);
        }
    }

    // 3. <task>
    let task_marker = confidence
        .and_then(|c| c.iter().find(|ic| ic.section == "task" && ic.index == 0))
        .and_then(|ic| Some(confidence_marker(ic.score, threshold)))
        
        .unwrap_or_default();
    let task_escaped = xml_escape(output.task.trim());
    sections.push(format!("<task>\n{}{}\n</task>", task_marker, task_escaped));

    // 4. <acceptance_criteria>
    let ac_items: Vec<String> = output
        .acceptance_criteria
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let trimmed = a.trim();
            let escaped = xml_escape(trimmed);
            let marker = confidence
                .and_then(|c| c.iter().find(|ic| ic.section == "acceptance_criteria" && ic.index == i))
                .and_then(|ic| Some(confidence_marker(ic.score, threshold)))
                
                .unwrap_or_default();
            if escaped.starts_with("- ") || escaped.starts_with("* ") {
                format!("{}{}", marker, escaped)
            } else {
                format!("{}- {}", marker, escaped)
            }
        })
        .collect();
    if !ac_items.is_empty() {
        sections.push(format!(
            "<acceptance_criteria>\n{}\n</acceptance_criteria>",
            ac_items.join("\n")
        ));
    }

    // 5. <context>
    if !output.context.is_empty() {
        let items: Vec<String> = output
            .context
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let trimmed = c.trim();
                let escaped = xml_escape(trimmed);
                let marker = confidence
                    .and_then(|c| c.iter().find(|ic| ic.section == "context" && ic.index == i))
                    .and_then(|ic| Some(confidence_marker(ic.score, threshold)))
                    
                    .unwrap_or_default();
                if escaped.starts_with("- ") || escaped.starts_with("* ") {
                    format!("{}{}", marker, escaped)
                } else {
                    format!("{}- {}", marker, escaped)
                }
            })
            .collect();
        sections.push(format!("<context>\n{}\n</context>", items.join("\n")));
    }

    // 6. <scope> = relevant_files + out_of_scope (omit if both empty)
    let scope_is_empty = output.relevant_files.is_empty() && output.out_of_scope.is_empty();
    if !scope_is_empty {
        let mut scope_parts = Vec::new();
        if !output.relevant_files.is_empty() {
            let files: Vec<String> = output
                .relevant_files
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let escaped = xml_escape(f.trim());
                    let marker = confidence
                        .and_then(|c| c.iter().find(|ic| ic.section == "scope" && ic.index == i))
                        .and_then(|ic| Some(confidence_marker(ic.score, threshold)))
                        
                        .unwrap_or_default();
                    format!("{}{}", marker, escaped)
                })
                .collect();
            scope_parts.push(format!("<relevant_files>{}</relevant_files>", files.join(", ")));
        }
        if !output.out_of_scope.is_empty() {
            let items: Vec<String> = output
                .out_of_scope
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let escaped = xml_escape(s.trim());
                    let idx = output.relevant_files.len() + i;
                    let marker = confidence
                        .and_then(|c| c.iter().find(|ic| ic.section == "scope" && ic.index == idx))
                        .and_then(|ic| Some(confidence_marker(ic.score, threshold)))
                        
                        .unwrap_or_default();
                    format!("{}{}", marker, escaped)
                })
                .collect();
            scope_parts.push(format!("<out_of_scope>{}</out_of_scope>", items.join(", ")));
        }
        sections.push(format!("<scope>\n{}\n</scope>", scope_parts.join("\n")));
    }

    // 7. <rules> (fixed profile rules + extra_rules)
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

    // 8. <verify> = LLM verify items (omit if empty)
    if !output.verify.is_empty() {
        let items: Vec<String> = output
            .verify
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let escaped = xml_escape(s.trim());
                let marker = confidence
                    .and_then(|c| c.iter().find(|ic| ic.section == "verify" && ic.index == i))
                    .and_then(|ic| Some(confidence_marker(ic.score, threshold)))
                    
                    .unwrap_or_default();
                format!("{}{}", marker, escaped)
            })
            .collect();
        sections.push(format!("<verify>\n{}\n</verify>", items.join("\n")));
    }

    // 9. <report> (fixed profile report + extra_report)
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
