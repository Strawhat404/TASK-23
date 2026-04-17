use serde::Deserialize;

/// Represents a single question parsed from a CSV row.
#[derive(Debug, Clone, Deserialize)]
pub struct CsvQuestion {
    pub question_text: String,
    pub option_a: String,
    pub option_b: String,
    pub option_c: String,
    pub option_d: String,
    pub correct_answer: String,
    pub difficulty: String,
    pub explanation: String,
}

/// Parse CSV content into a list of `CsvQuestion` values.
///
/// Expected columns: question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
///
/// Returns `Ok(questions)` on success, or `Err(errors)` listing per-row problems.
pub fn parse_csv(content: &str) -> Result<Vec<CsvQuestion>, Vec<String>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(content.as_bytes());

    let mut questions = Vec::new();
    let mut errors = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let row_num = idx + 2; // 1-indexed, plus header row
        match result {
            Ok(record) => {
                if record.len() < 8 {
                    errors.push(format!("Row {}: expected 8 columns, got {}", row_num, record.len()));
                    continue;
                }

                let question_text = record.get(0).unwrap_or("").trim().to_string();
                let option_a = record.get(1).unwrap_or("").trim().to_string();
                let option_b = record.get(2).unwrap_or("").trim().to_string();
                let option_c = record.get(3).unwrap_or("").trim().to_string();
                let option_d = record.get(4).unwrap_or("").trim().to_string();
                let correct_answer = record.get(5).unwrap_or("").trim().to_uppercase();
                let difficulty = record.get(6).unwrap_or("medium").trim().to_lowercase();
                let explanation = record.get(7).unwrap_or("").trim().to_string();

                if question_text.is_empty() {
                    errors.push(format!("Row {}: question text is empty", row_num));
                    continue;
                }

                if !["a", "b", "c", "d", "ab", "ac", "ad", "bc", "bd", "cd", "abc", "abd", "acd", "bcd", "abcd"]
                    .contains(&correct_answer.to_lowercase().as_str())
                {
                    errors.push(format!(
                        "Row {}: invalid correct answer '{}', expected one of A/B/C/D or combination",
                        row_num, correct_answer
                    ));
                    continue;
                }

                if !["easy", "medium", "hard"].contains(&difficulty.as_str()) {
                    errors.push(format!(
                        "Row {}: invalid difficulty '{}', expected easy/medium/hard",
                        row_num, difficulty
                    ));
                    continue;
                }

                questions.push(CsvQuestion {
                    question_text,
                    option_a,
                    option_b,
                    option_c,
                    option_d,
                    correct_answer,
                    difficulty,
                    explanation,
                });
            }
            Err(e) => {
                errors.push(format!("Row {}: parse error: {}", row_num, e));
            }
        }
    }

    if questions.is_empty() && !errors.is_empty() {
        Err(errors)
    } else {
        Ok(questions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── happy-path parsing ───────────────────────────────────────────────────

    #[test]
    fn parses_well_formed_csv_single_row() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
What is 2+2?,1,2,3,4,D,easy,Simple arithmetic
";
        let rows = parse_csv(csv).expect("valid csv must parse");
        assert_eq!(rows.len(), 1);
        let q = &rows[0];
        assert_eq!(q.question_text, "What is 2+2?");
        assert_eq!(q.option_a, "1");
        assert_eq!(q.option_d, "4");
        assert_eq!(q.correct_answer, "D");
        assert_eq!(q.difficulty, "easy");
        assert_eq!(q.explanation, "Simple arithmetic");
    }

    #[test]
    fn parses_multiple_rows() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
Q1?,A,B,C,D,A,easy,e1
Q2?,A,B,C,D,B,medium,e2
Q3?,A,B,C,D,C,hard,e3
";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].difficulty, "easy");
        assert_eq!(rows[1].difficulty, "medium");
        assert_eq!(rows[2].difficulty, "hard");
    }

    #[test]
    fn multi_select_correct_answers_are_accepted() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
Which are even?,1,2,3,4,BD,medium,2 and 4
";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows[0].correct_answer, "BD");
    }

    #[test]
    fn abcd_multi_select_is_accepted() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
All true?,T,T,T,T,ABCD,hard,everything
";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows[0].correct_answer, "ABCD");
    }

    // ── normalisation ────────────────────────────────────────────────────────

    #[test]
    fn correct_answer_is_uppercased() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
Q?,1,2,3,4,b,easy,e
";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows[0].correct_answer, "B");
    }

    #[test]
    fn difficulty_is_lowercased() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
Q?,1,2,3,4,A,HARD,e
";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows[0].difficulty, "hard");
    }

    #[test]
    fn whitespace_is_trimmed() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
  padded  ,  a  ,  b  ,  c  ,  d  ,  A  ,  easy  ,  expl
";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows[0].question_text, "padded");
        assert_eq!(rows[0].option_a, "a");
        assert_eq!(rows[0].explanation, "expl");
    }

    // ── validation errors ────────────────────────────────────────────────────

    #[test]
    fn invalid_correct_answer_is_rejected() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
Q?,1,2,3,4,Z,easy,e
";
        // Entire valid-question set is empty → returns Err with row error.
        let err = parse_csv(csv).unwrap_err();
        assert!(err.iter().any(|e| e.contains("invalid correct answer")));
    }

    #[test]
    fn invalid_difficulty_is_rejected() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
Q?,1,2,3,4,A,impossible,e
";
        let err = parse_csv(csv).unwrap_err();
        assert!(err.iter().any(|e| e.contains("invalid difficulty")));
    }

    #[test]
    fn empty_question_text_is_rejected() {
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
,a,b,c,d,A,easy,e
";
        let err = parse_csv(csv).unwrap_err();
        assert!(err.iter().any(|e| e.contains("question text is empty")));
    }

    #[test]
    fn partial_valid_rows_are_returned_with_mix() {
        // One valid row, one bad row → Ok with the valid row (errors are lost
        // in this mixed case because the fn returns Ok when questions isn't
        // empty). This exercises the intended "best effort import" behaviour.
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
Good?,a,b,c,d,A,easy,ok
,a,b,c,d,A,easy,empty
";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].question_text, "Good?");
    }

    #[test]
    fn empty_csv_returns_empty_vec() {
        let csv = "question,option_a,option_b,option_c,option_d,correct,difficulty,explanation\n";
        let rows = parse_csv(csv).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn malformed_row_reports_error() {
        // Row with fewer columns than the header expects.
        let csv = "\
question,option_a,option_b,option_c,option_d,correct,difficulty,explanation
just,three,cols
";
        // CSV reader is strict about column count — this yields an Err.
        let result = parse_csv(csv);
        assert!(result.is_err(), "short row must produce error");
    }
}
