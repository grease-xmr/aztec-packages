
/// A toy example that multiplies an input by a factor, but we don't know which one until the factor is revealed.
fn main(inputs: pub [u64; 4], index: u32, offset: pub u64, factor: u64) -> pub u64 {
    let factor = factor as Field;
    let used = inputs[index] as Field;
    let result = used * factor + offset as Field;

    result as u64
}

#[test]
fn test_main() {
    let output = main([1,2,3,4], 2, 5, 10);
    assert_eq(output, 35);
}
