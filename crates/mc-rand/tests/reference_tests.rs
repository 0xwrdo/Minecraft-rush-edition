//! Integration test reading expected reference outputs from tests/java_reference/expected_random.csv
//! and validating exact parity against Java's java.util.Random.

use mc_rand::JavaRandom;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[test]
fn test_against_java_reference_dataset() {
    let csv_path = Path::new("tests/java_reference/expected_random.csv");
    let file = File::open(csv_path)
        .expect("expected_random.csv not found; ensure GenerateReference has run");
    let reader = BufReader::new(file);

    let mut current_seed: Option<i64> = None;
    let mut rng = JavaRandom::new(0);

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.expect("Failed to read line from CSV");
        if line_num == 0 || line.trim().is_empty() {
            continue; // Header or blank line
        }

        let parts: Vec<&str> = line.split(',').collect();
        assert_eq!(
            parts.len(),
            4,
            "Invalid CSV format at line {}",
            line_num + 1
        );

        let seed: i64 = parts[0].parse().expect("Invalid seed integer");
        let op = parts[1];
        let _idx: usize = parts[2].parse().expect("Invalid index");
        let expected_str = parts[3];

        if current_seed != Some(seed) {
            current_seed = Some(seed);
            rng = JavaRandom::new(seed);
        }

        match op {
            "nextInt" => {
                let expected: i32 = expected_str.parse().unwrap();
                let actual = rng.next_int();
                assert_eq!(
                    actual,
                    expected,
                    "nextInt mismatch at line {}",
                    line_num + 1
                );
            }
            s if s.starts_with("nextIntBound") => {
                let bound_str = &s["nextIntBound".len()..];
                let bound: i32 = bound_str.parse().unwrap();
                let expected: i32 = expected_str.parse().unwrap();
                let actual = rng.next_int_bounded(bound);
                assert_eq!(
                    actual,
                    expected,
                    "nextInt_bounded({}) mismatch at line {}",
                    bound,
                    line_num + 1
                );
            }
            "nextLong" => {
                let expected: i64 = expected_str.parse().unwrap();
                let actual = rng.next_long();
                assert_eq!(
                    actual,
                    expected,
                    "nextLong mismatch at line {}",
                    line_num + 1
                );
            }
            "nextBoolean" => {
                let expected: bool = expected_str.parse().unwrap();
                let actual = rng.next_boolean();
                assert_eq!(
                    actual,
                    expected,
                    "nextBoolean mismatch at line {}",
                    line_num + 1
                );
            }
            "nextFloat" => {
                let expected: f32 = expected_str.parse().unwrap();
                let actual = rng.next_float();
                assert!(
                    (actual - expected).abs() < 1e-6,
                    "nextFloat mismatch at line {}: actual {}, expected {}",
                    line_num + 1,
                    actual,
                    expected
                );
            }
            "nextDouble" => {
                let expected: f64 = expected_str.parse().unwrap();
                let actual = rng.next_double();
                assert!(
                    (actual - expected).abs() < 1e-15,
                    "nextDouble mismatch at line {}: actual {}, expected {}",
                    line_num + 1,
                    actual,
                    expected
                );
            }
            "nextGaussian" => {
                let expected: f64 = expected_str.parse().unwrap();
                let actual = rng.next_gaussian();
                assert!(
                    (actual - expected).abs() < 1e-15,
                    "nextGaussian mismatch at line {}: actual {}, expected {}",
                    line_num + 1,
                    actual,
                    expected
                );
            }
            other => panic!("Unknown operation '{}' at line {}", other, line_num + 1),
        }
    }
}
