use regex_lite::Regex;
use std::sync::OnceLock;

static STEP1_STRICT: OnceLock<Regex> = OnceLock::new();
static STEP1_LOOSE: OnceLock<Regex> = OnceLock::new();
static STEP2_STRICT: OnceLock<Regex> = OnceLock::new();
static STEP2_LOOSE: OnceLock<Regex> = OnceLock::new();
static STEP3_STRICT: OnceLock<Regex> = OnceLock::new();
static STEP3_LOOSE: OnceLock<Regex> = OnceLock::new();

fn init_regexes() -> (
    &'static Regex,
    &'static Regex,
    &'static Regex,
    &'static Regex,
    &'static Regex,
    &'static Regex,
) {
    let s1_s = STEP1_STRICT.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)\b(?:jev|decider|gate)\s+(?:must\s+|first\s+|will\s+|should\s+|always\s+)?(?:find|identif\w*|determin\w*|detect\w*|classif\w*|resolv\w*|discover\w*|select\w*)\b[\s\S]*?\b(?:prompt|session|task|template|input)\s+(?:type|kind|category)\b",
            r"|",
            r"(?i)\b(?:find|identif\w*|determin\w*|detect\w*|classif\w*|resolv\w*|discover\w*|select\w*)\b[\s\S]*?\b(?:prompt|session|task|template|input)\s+(?:type|kind|category)\b[\s\S]*?\b(?:by|via)\s+(?:jev|decider|gate)\b"
        )).expect("valid regex for step 1 strict")
    });

    let s1_l = STEP1_LOOSE.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)\b(?:find|identif\w*|determin\w*|detect\w*|classif\w*|resolv\w*|discover\w*|select\w*)\b[\s\S]*?\b(?:prompt|session|task|template|input)\s+(?:type|kind|category)\b",
            r"|",
            r"(?i)\b(?:jev|decider|gate)\b[\s\S]*?\b(?:prompt|session|task|template|input)\s+(?:type|kind|category)\b"
        )).expect("valid regex for step 1 loose")
    });

    let s2_s = STEP2_STRICT.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)\b(?:jev|decider|gate)\s+(?:then\s+|will\s+|should\s+|next\s+|also\s+)?(?:tells?|instruct\w*|direct\w*|asks?|prompts?|calls?|delegat\w*|tasks?|requests?)\b[\s\S]*?\b(?:the\s+)?(?:llm|model|writer|generator|agent|ai)\b[\s\S]*?\b(?:to\s+)?(?:create|generat\w*|writ\w*|build\w*|produc\w*|mak\w*)\b[\s\S]*?\bdynamic\b",
            r"|",
            r"(?i)\b(?:the\s+)?(?:llm|model|writer|generator|agent|ai)\b[\s\S]*?\b(?:creates?|generat\w*|writ\w*|build\w*|produc\w*)\b[\s\S]*?\bdynamic\b[\s\S]*?\b(?:instructed\s+by|from|via)\s+(?:jev|decider|gate)\b"
        )).expect("valid regex for step 2 strict")
    });

    let s2_l = STEP2_LOOSE.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)\b(?:the\s+)?(?:llm|model|writer|generator|agent|ai)\b[\s\S]*?\b(?:to\s+)?(?:create|generat\w*|writ\w*|build\w*|produc\w*|mak\w*)\b[\s\S]*?\bdynamic\b",
            r"|",
            r"(?i)\b(?:jev|decider|gate)\b[\s\S]*?\b(?:create|generat\w*|writ\w*|build\w*|produc\w*)\b[\s\S]*?\bdynamic\b",
            r"|",
            r"(?i)\b(?:create|generat\w*|writ\w*|build\w*|produc\w*)\b[\s\S]*?\bdynamic\s+(?:parts?|sections?|content|elements?|fields?)\b"
        )).expect("valid regex for step 2 loose")
    });

    let s3_s = STEP3_STRICT.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)\b(?:jev|decider|gate|pipeline)\s+(?:then\s+|also\s+|will\s+|should\s+|finally\s+)?(?:completes?|fills?|adds?|inject\w*|populat\w*|appends?|finaliz\w*|inserts?|renders?)\b[\s\S]*?\b(?:any\s+)?static\b[\s\S]*?\b(?:sections?|parts?|blocks?|content|templates?|rules?|elements?)\b",
            r"|",
            r"(?i)\b(?:completes?|fills?|adds?|inject\w*|populat\w*|appends?|finaliz\w*|inserts?|renders?)\b[\s\S]*?\b(?:any\s+)?static\b[\s\S]*?\b(?:sections?|parts?|blocks?|content|templates?|rules?|elements?)\b[\s\S]*?\b(?:by|via)\s+(?:jev|decider|gate|pipeline)\b"
        )).expect("valid regex for step 3 strict")
    });

    let s3_l = STEP3_LOOSE.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)\b(?:completes?|fills?|adds?|inject\w*|populat\w*|appends?|finaliz\w*|inserts?|renders?)\b[\s\S]*?\b(?:any\s+)?static\b[\s\S]*?\b(?:sections?|parts?|blocks?|content|templates?|rules?|elements?)\b",
            r"|",
            r"(?i)\bstatic\b[\s\S]*?\b(?:sections?|parts?|blocks?|content|templates?|rules?|elements?)\b[\s\S]*?\b(?:completed|filled|added|injected|populated)\b"
        )).expect("valid regex for step 3 loose")
    });

    (s1_s, s1_l, s2_s, s2_l, s3_s, s3_l)
}

/// Validates whether a textual description of a workflow matches the expected sequence:
/// (1) Jev must find the prompt type,
/// (2) Jev tells the LLM to create dynamic parts, and
/// (3) Jev completes any static sections if needed.
///
/// Returns:
/// - `"Match"`: All 3 steps present in exact order (1 -> 2 -> 3) using synonymous phrasing.
/// - `"Near match"`: All 3 steps present, but order differs or wording varies moderately.
/// - `"Far away"`: One or more steps missing, or unrelated content dominates.
pub fn validate_workflow(description: &str) -> &'static str {
    let trimmed = description.trim();
    if trimmed.is_empty() {
        return "Far away";
    }

    let (s1_s, s1_l, s2_s, s2_l, s3_s, s3_l) = init_regexes();

    let m1_strict = s1_s.find(trimmed);
    let m1_match = m1_strict.or_else(|| s1_l.find(trimmed));
    let m1 = match m1_match {
        Some(m) => m,
        None => return "Far away",
    };

    let m2_strict = s2_s.find(trimmed);
    let m2_match = m2_strict.or_else(|| s2_l.find(trimmed));
    let m2 = match m2_match {
        Some(m) => m,
        None => return "Far away",
    };

    let m3_strict = s3_s.find(trimmed);
    let m3_match = m3_strict.or_else(|| s3_l.find(trimmed));
    let m3 = match m3_match {
        Some(m) => m,
        None => return "Far away",
    };

    let pos1 = m1.start();
    let pos2 = m2.start();
    let pos3 = m3.start();

    let in_order = pos1 < pos2 && pos2 < pos3;
    let all_strict = m1_strict.is_some() && m2_strict.is_some() && m3_strict.is_some();

    if in_order && all_strict {
        "Match"
    } else {
        "Near match"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_verbatim() {
        let input = "(1) Jev must find the prompt type, (2) Jev tells the LLM to create dynamic parts, and (3) Jev completes any static sections if needed.";
        assert_eq!(validate_workflow(input), "Match");
    }

    #[test]
    fn test_exact_match_synonymous_in_order() {
        let input = "First, Jev determines the prompt type. Next, Jev instructs the LLM to generate dynamic sections. Finally, Jev populates any static sections if required.";
        assert_eq!(validate_workflow(input), "Match");

        let input2 = "1. Jev determines the session kind\n2. Jev directs the model to build dynamic parts\n3. Jev completes static sections as needed";
        assert_eq!(validate_workflow(input2), "Match");

        let input3 = "The workflow begins when Jev classifies the prompt category, after which Jev asks the writer to produce dynamic parts, and then Jev completes static sections.";
        assert_eq!(validate_workflow(input3), "Match");
    }

    #[test]
    fn test_near_match_out_of_order() {
        // Steps 2, 1, 3
        let input = "First, Jev tells the LLM to create dynamic parts. Before that, Jev must find the prompt type. Finally, Jev completes static sections if needed.";
        assert_eq!(validate_workflow(input), "Near match");

        // Steps 3, 2, 1
        let input2 = "Jev completes any static sections if needed, after Jev tells the LLM to create dynamic parts, and initially Jev finds the prompt type.";
        assert_eq!(validate_workflow(input2), "Near match");

        // Steps 1, 3, 2
        let input3 = "Jev finds the prompt type, then Jev completes static sections, and then Jev tells the LLM to create dynamic parts.";
        assert_eq!(validate_workflow(input3), "Near match");
    }

    #[test]
    fn test_near_match_moderate_wording_variation() {
        // Actor Jev omitted in steps, but all 3 steps present in order
        let input = "Step 1: Determine prompt type. Step 2: The LLM generates dynamic parts. Step 3: Complete static sections if needed.";
        assert_eq!(validate_workflow(input), "Near match");
    }

    #[test]
    fn test_far_away_missing_steps() {
        // Missing step 3
        let input = "1. Jev finds the prompt type. 2. Jev tells the LLM to create dynamic parts.";
        assert_eq!(validate_workflow(input), "Far away");

        // Missing step 1
        let input2 = "Jev tells the LLM to create dynamic parts and completes static sections.";
        assert_eq!(validate_workflow(input2), "Far away");

        // Missing step 2
        let input3 = "Jev finds the prompt type and completes static sections.";
        assert_eq!(validate_workflow(input3), "Far away");
    }

    #[test]
    fn test_far_away_unrelated_content() {
        let input = "Deploy the application to AWS ECS and check CloudWatch logs.";
        assert_eq!(validate_workflow(input), "Far away");

        let input2 = "The quick brown fox jumps over the lazy dog.";
        assert_eq!(validate_workflow(input2), "Far away");

        let input3 = "";
        assert_eq!(validate_workflow(input3), "Far away");
    }
}
