/// Unit tests for helper functions in src/misc/helpers.rs
use scilla::misc::helpers::lamports_to_sol;

#[test]
fn test_lamports_to_sol_zero() {
    let result = lamports_to_sol(0);
    assert_eq!(result, 0.0, "0 lamports should convert to 0.0 SOL");
}

#[test]
fn test_lamports_to_sol_one_lamport() {
    let result = lamports_to_sol(1);
    let expected = 1.0 / 1_000_000_000.0;
    assert!(
        (result - expected).abs() < 1e-15,
        "1 lamport should be 0.000000001 SOL, got {}",
        result
    );
}

#[test]
fn test_lamports_to_sol_one_sol() {
    let result = lamports_to_sol(1_000_000_000);
    assert_eq!(result, 1.0, "1 billion lamports should be exactly 1.0 SOL");
}

#[test]
fn test_lamports_to_sol_fractional() {
    let result = lamports_to_sol(500_000_000);
    assert_eq!(result, 0.5, "500 million lamports should be 0.5 SOL");
}

#[test]
fn test_lamports_to_sol_large_amount() {
    let result = lamports_to_sol(12_345_678_900);
    let expected = 12.3456789;
    assert!(
        (result - expected).abs() < 1e-10,
        "12.3456789 billion lamports should be 12.3456789 SOL, got {}",
        result
    );
}

#[test]
fn test_lamports_to_sol_max_u64() {
    // u64::MAX = 18,446,744,073,709,551,615
    // Divided by 1e9 = ~18,446,744,073.709551615 SOL
    let result = lamports_to_sol(u64::MAX);
    assert!(
        result > 18_446_744_073.0 && result < 18_446_744_074.0,
        "u64::MAX lamports should be ~18.4 billion SOL"
    );
}

#[test]
fn test_lamports_to_sol_precision() {
    // Test that we maintain precision for typical wallet balances
    let result = lamports_to_sol(1_234_567_890);
    let expected = 1.23456789;
    assert!(
        (result - expected).abs() < 1e-10,
        "Should maintain 8 decimal places of precision, got {} expected {}",
        result,
        expected
    );
}

#[test]
fn test_lamports_to_sol_table_driven() {
    let test_cases = vec![
        (0, 0.0),
        (1, 0.000000001),
        (1_000, 0.000001),
        (1_000_000, 0.001),
        (1_000_000_000, 1.0),
        (2_500_000_000, 2.5),
        (10_000_000_000, 10.0),
    ];

    for (lamports, expected_sol) in test_cases {
        let result = lamports_to_sol(lamports);
        assert!(
            (result - expected_sol).abs() < 1e-10,
            "lamports_to_sol({}) = {}, expected {}",
            lamports,
            result,
            expected_sol
        );
    }
}
